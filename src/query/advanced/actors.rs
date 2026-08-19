//! Query execution engine actors.
//!
//! Phase 3: Converts five sequential `std::sync` lock acquisitions per query
//! into four independent actor tasks communicating via `tokio::mpsc` channels.
//! Each actor owns its state directly and processes messages in a
//! `tokio::select!` loop — no `Arc<Mutex<>>` or `Arc<RwLock<>>` needed.

#![allow(dead_code)]

use super::conjunctive::ConjunctiveQuery;
use super::cost_optimizer::CostBasedOptimizer;
use super::execution::{AdvancedQueryError, ConjunctiveQueryResult};
use super::execution_engine::{
    CacheConfig, ExecutionConstraints, ExecutionContext, ExecutionId, ExecutionPerformanceMonitor,
    ExecutionStrategySelector, ParallelExecutionConfig, ParallelTask, QueryResultCache,
    ResourceManager, TaskId, TaskStatus, ThreadPool,
};
use super::ml_core::{
    ExecutionStrategy as MLExecutionStrategy, MLHeuristicsEngine as MLEngine, QueryExecution,
    StrategyRecommendation,
};
use super::optimizer::AdvancedQueryPlan;
use crate::ontology::Ontology;
use crate::reasoning::ReasoningService;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use tokio::select;
use tokio::sync::{mpsc, oneshot};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn map_strategy_name(name: &str) -> MLExecutionStrategy {
    match name {
        "indexed_lookup" => MLExecutionStrategy::IndexedLookup,
        "join_order" => MLExecutionStrategy::JoinOrder,
        "materialization" => MLExecutionStrategy::Materialization,
        "hybrid" => MLExecutionStrategy::Hybrid,
        "backward_chaining" => MLExecutionStrategy::BackwardChaining,
        "forward_chaining" => MLExecutionStrategy::ForwardChaining,
        "parallel" => MLExecutionStrategy::Parallel,
        "adaptive" => MLExecutionStrategy::Adaptive,
        _ => MLExecutionStrategy::Default,
    }
}

// ─── OptimizerActor ──────────────────────────────────────────────────────────

/// Messages processed by the `OptimizerActor`.
pub enum OptimizerMsg {
    /// Generate an optimized query plan.
    OptimizeQuery {
        query: ConjunctiveQuery,
        reply: oneshot::Sender<Result<AdvancedQueryPlan, AdvancedQueryError>>,
    },
    /// Select a strategy name via the legacy rule-based selector.
    SelectStrategy {
        query: ConjunctiveQuery,
        plan: AdvancedQueryPlan,
        reply: oneshot::Sender<Result<String, AdvancedQueryError>>,
    },
    /// Execute a query sequentially and return the result.
    ExecuteSequential {
        query: ConjunctiveQuery,
        strategy: String,
        constraints: ExecutionConstraints,
        reply: oneshot::Sender<Result<ConjunctiveQueryResult, AdvancedQueryError>>,
    },
    /// Fire-and-forget: update strategy performance history.
    UpdateHistory {
        strategy: String,
        query: Box<ConjunctiveQuery>,
        result: Box<ConjunctiveQueryResult>,
    },
    /// Graceful shutdown.
    Shutdown,
}

/// Handle to the actor that owns [`CostBasedOptimizer`] + [`ExecutionStrategySelector`].
///
/// Only one tokio task ever touches either component — no locking required.
pub struct OptimizerHandle {
    tx: mpsc::Sender<OptimizerMsg>,
}

impl OptimizerHandle {
    /// Spawn the actor task and return a handle.
    pub fn spawn(
        optimizer: CostBasedOptimizer,
        strategy_selector: ExecutionStrategySelector,
        ontology: Arc<Ontology>,
        reasoning_service: Arc<ReasoningService>,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<OptimizerMsg>(64);
        tokio::spawn(async move {
            let mut optimizer = optimizer;
            let mut selector = strategy_selector;
            loop {
                select! {
                    msg = rx.recv() => match msg {
                        Some(OptimizerMsg::OptimizeQuery { query, reply }) => {
                            let res = optimizer
                                .optimize_query(&query)
                                .map_err(AdvancedQueryError::from);
                            let _ = reply.send(res);
                        }
                        Some(OptimizerMsg::SelectStrategy { query, plan, reply }) => {
                            let res = selector.select_strategy(&query, &plan);
                            let _ = reply.send(res);
                        }
                        Some(OptimizerMsg::ExecuteSequential { query, strategy, constraints, reply }) => {
                            let result = (|| -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
                                let context = ExecutionContext {
                                    ontology: ontology.clone(),
                                    reasoning_service: reasoning_service.clone(),
                                    available_indices: Vec::new(),
                                    constraints,
                                    cache: Arc::new(RwLock::new(
                                        QueryResultCache::new(CacheConfig::default()),
                                    )),
                                };
                                let strategy_impl = selector.get_strategy(&strategy)?;
                                strategy_impl.execute(&query, &context)
                            })();
                            let _ = reply.send(result);
                        }
                        Some(OptimizerMsg::UpdateHistory { strategy, query, result }) => {
                            selector.update_performance_history(&strategy, &query, &result);
                        }
                        Some(OptimizerMsg::Shutdown) | None => break,
                    }
                }
            }
        });
        Self { tx }
    }

    /// Generate an optimized plan for `query`.
    pub async fn optimize_query(
        &self,
        query: ConjunctiveQuery,
    ) -> Result<AdvancedQueryPlan, AdvancedQueryError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(OptimizerMsg::OptimizeQuery { query, reply: tx })
            .await
            .map_err(|_| AdvancedQueryError::InternalError("OptimizerActor dead".into()))?;
        rx.await
            .map_err(|_| AdvancedQueryError::InternalError("OptimizerActor reply failed".into()))?
    }

    /// Select a strategy name using the legacy rule-based selector.
    pub async fn select_strategy(
        &self,
        query: ConjunctiveQuery,
        plan: AdvancedQueryPlan,
    ) -> Result<String, AdvancedQueryError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(OptimizerMsg::SelectStrategy {
                query,
                plan,
                reply: tx,
            })
            .await
            .map_err(|_| AdvancedQueryError::InternalError("OptimizerActor dead".into()))?;
        rx.await
            .map_err(|_| AdvancedQueryError::InternalError("OptimizerActor reply failed".into()))?
    }

    /// Execute a query sequentially inside the actor task.
    pub async fn execute_sequential(
        &self,
        query: ConjunctiveQuery,
        strategy: String,
        constraints: ExecutionConstraints,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(OptimizerMsg::ExecuteSequential {
                query,
                strategy,
                constraints,
                reply: tx,
            })
            .await
            .map_err(|_| AdvancedQueryError::InternalError("OptimizerActor dead".into()))?;
        rx.await
            .map_err(|_| AdvancedQueryError::InternalError("OptimizerActor reply failed".into()))?
    }

    /// Fire-and-forget: update performance history after a completed execution.
    pub async fn update_history(
        &self,
        strategy: String,
        query: ConjunctiveQuery,
        result: ConjunctiveQueryResult,
    ) {
        let _ = self
            .tx
            .send(OptimizerMsg::UpdateHistory {
                strategy,
                query: Box::new(query),
                result: Box::new(result),
            })
            .await;
    }
}

// ─── MLStrategyActor ─────────────────────────────────────────────────────────

/// Messages processed by the `MLStrategyActor`.
pub enum MLMsg {
    /// Select a strategy using the ML recommendation engine.
    SelectStrategy {
        query: ConjunctiveQuery,
        ontology: Arc<Ontology>,
        reply:
            oneshot::Sender<Result<(String, Option<StrategyRecommendation>), AdvancedQueryError>>,
    },
    /// Fire-and-forget: add an execution record for online learning.
    ProvideFeedback {
        query: ConjunctiveQuery,
        ontology: Arc<Ontology>,
        strategy_used: String,
        actual_time_secs: f64,
        actual_memory_mb: f64,
    },
    /// Graceful shutdown.
    Shutdown,
}

/// Handle to the actor that owns [`MLEngine`].
pub struct MLStrategyHandle {
    tx: mpsc::Sender<MLMsg>,
}

impl MLStrategyHandle {
    /// Spawn the actor task and return a handle.
    pub fn spawn(ml_engine: MLEngine) -> Self {
        let (tx, mut rx) = mpsc::channel::<MLMsg>(64);
        tokio::spawn(async move {
            let ml_engine = ml_engine;
            loop {
                select! {
                    msg = rx.recv() => match msg {
                        Some(MLMsg::SelectStrategy { query, ontology, reply }) => {
                            let result = (|| -> Result<(String, Option<StrategyRecommendation>), AdvancedQueryError> {
                                let features = ml_engine
                                    .extract_features(&query, &ontology)
                                    .map_err(|e| AdvancedQueryError::InternalError(
                                        format!("Feature extraction failed: {e}"),
                                    ))?;
                                let recommendation = ml_engine
                                    .select_strategy(&features)
                                    .map_err(|e| AdvancedQueryError::InternalError(
                                        format!("Strategy selection failed: {e}"),
                                    ))?;
                                let name = recommendation.strategy.as_str().to_string();
                                Ok((name, Some(recommendation)))
                            })();
                            let _ = reply.send(result);
                        }
                        Some(MLMsg::ProvideFeedback {
                            query,
                            ontology,
                            strategy_used,
                            actual_time_secs,
                            actual_memory_mb,
                        }) => {
                            if let Ok(features) = ml_engine.extract_features(&query, &ontology) {
                                let execution = QueryExecution {
                                    features,
                                    actual_time: actual_time_secs,
                                    actual_memory: actual_memory_mb,
                                    strategy_used: map_strategy_name(&strategy_used),
                                };
                                let _ = ml_engine.add_training_data(execution);
                            }
                        }
                        Some(MLMsg::Shutdown) | None => break,
                    }
                }
            }
        });
        Self { tx }
    }

    /// Select a strategy using ML and return its name + recommendation.
    pub async fn select_strategy(
        &self,
        query: ConjunctiveQuery,
        ontology: Arc<Ontology>,
    ) -> Result<(String, Option<StrategyRecommendation>), AdvancedQueryError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(MLMsg::SelectStrategy {
                query,
                ontology,
                reply: tx,
            })
            .await
            .map_err(|_| AdvancedQueryError::InternalError("MLStrategyActor dead".into()))?;
        rx.await
            .map_err(|_| AdvancedQueryError::InternalError("MLStrategyActor reply failed".into()))?
    }

    /// Fire-and-forget: feed back execution data for online learning.
    pub async fn provide_feedback(
        &self,
        query: ConjunctiveQuery,
        ontology: Arc<Ontology>,
        strategy_used: String,
        actual_time_secs: f64,
        actual_memory_mb: f64,
    ) {
        let _ = self
            .tx
            .send(MLMsg::ProvideFeedback {
                query,
                ontology,
                strategy_used,
                actual_time_secs,
                actual_memory_mb,
            })
            .await;
    }
}

// ─── MonitorActor (fire-and-forget telemetry) ─────────────────────────────────

/// Messages processed by the `MonitorActor`.
pub enum MonitorMsg {
    /// Record the start of a query execution.
    StartExecution {
        execution_id: ExecutionId,
        query: Box<ConjunctiveQuery>,
        strategy: String,
    },
    /// Record the completion of a query execution.
    CompleteExecution {
        execution_id: ExecutionId,
        /// `Ok(result)` on success or `Err(error_string)` on failure.
        outcome: Result<ConjunctiveQueryResult, String>,
    },
    /// Graceful shutdown.
    Shutdown,
}

/// Fire-and-forget handle to the performance monitor actor.
///
/// Uses an unbounded channel — telemetry must never block callers.
pub struct MonitorHandle {
    tx: mpsc::UnboundedSender<MonitorMsg>,
}

impl MonitorHandle {
    /// Spawn the actor task and return a handle.
    pub fn spawn(monitor: ExecutionPerformanceMonitor) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<MonitorMsg>();
        tokio::spawn(async move {
            let mut monitor = monitor;
            loop {
                select! {
                    msg = rx.recv() => match msg {
                        Some(MonitorMsg::StartExecution { execution_id, query, strategy }) => {
                            monitor.start_execution(&execution_id, &query, &strategy);
                        }
                        Some(MonitorMsg::CompleteExecution { execution_id, outcome }) => {
                            let result: Result<ConjunctiveQueryResult, AdvancedQueryError> =
                                outcome.map_err(AdvancedQueryError::InternalError);
                            monitor.complete_execution(&execution_id, &result);
                        }
                        Some(MonitorMsg::Shutdown) | None => break,
                    }
                }
            }
        });
        Self { tx }
    }

    /// Fire-and-forget: notify actor that an execution started.
    pub fn start_execution(
        &self,
        execution_id: ExecutionId,
        query: ConjunctiveQuery,
        strategy: String,
    ) {
        let _ = self.tx.send(MonitorMsg::StartExecution {
            execution_id,
            query: Box::new(query),
            strategy,
        });
    }

    /// Fire-and-forget: notify actor that an execution completed.
    pub fn complete_execution(
        &self,
        execution_id: ExecutionId,
        outcome: Result<ConjunctiveQueryResult, String>,
    ) {
        let _ = self.tx.send(MonitorMsg::CompleteExecution {
            execution_id,
            outcome,
        });
    }
}

// ─── TaskCoordinatorActor ─────────────────────────────────────────────────────

/// Messages processed by the `TaskCoordinatorActor`.
pub enum TaskCoordinatorMsg {
    /// Submit a parallel task for execution.
    SubmitTask {
        task: ParallelTask,
        reply: oneshot::Sender<Result<TaskId, AdvancedQueryError>>,
    },
    /// Query the status of a submitted task.
    GetStatus {
        task_id: TaskId,
        reply: oneshot::Sender<Option<TaskStatus>>,
    },
    /// Cancel a running or queued task.
    CancelTask { task_id: TaskId },
    /// Graceful shutdown.
    Shutdown,
}

/// Handle to the actor that owns the parallel task coordinator state.
///
/// All state — thread pool, work queue, active tasks, resource manager — lives
/// inside the actor task with no external locking.
pub struct TaskCoordinatorHandle {
    tx: mpsc::Sender<TaskCoordinatorMsg>,
}

impl TaskCoordinatorHandle {
    /// Spawn the actor task and return a handle.
    pub fn spawn(
        thread_pool: ThreadPool,
        resource_manager: ResourceManager,
        config: ParallelExecutionConfig,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<TaskCoordinatorMsg>(256);
        tokio::spawn(async move {
            let _thread_pool = thread_pool;
            let _resource_manager = resource_manager;
            let _config = config;
            let mut work_queue: VecDeque<ParallelTask> = VecDeque::new();
            let mut active_tasks: HashMap<TaskId, TaskStatus> = HashMap::new();
            loop {
                select! {
                    msg = rx.recv() => match msg {
                        Some(TaskCoordinatorMsg::SubmitTask { task, reply }) => {
                            let task_id = task.task_id.clone();
                            active_tasks.insert(task_id.clone(), TaskStatus::Queued);
                            work_queue.push_back(task);
                            let _ = reply.send(Ok(task_id));
                        }
                        Some(TaskCoordinatorMsg::GetStatus { task_id, reply }) => {
                            let _ = reply.send(active_tasks.get(&task_id).cloned());
                        }
                        Some(TaskCoordinatorMsg::CancelTask { task_id }) => {
                            active_tasks.insert(
                                task_id,
                                TaskStatus::Cancelled {
                                    cancelled_at: std::time::Instant::now(),
                                },
                            );
                        }
                        Some(TaskCoordinatorMsg::Shutdown) | None => break,
                    }
                }
            }
        });
        Self { tx }
    }

    /// Submit a task and return its assigned `TaskId`.
    pub async fn submit_task(&self, task: ParallelTask) -> Result<TaskId, AdvancedQueryError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(TaskCoordinatorMsg::SubmitTask { task, reply: tx })
            .await
            .map_err(|_| AdvancedQueryError::InternalError("TaskCoordinatorActor dead".into()))?;
        rx.await.map_err(|_| {
            AdvancedQueryError::InternalError("TaskCoordinatorActor reply failed".into())
        })?
    }

    /// Query the current status of a task.
    pub async fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        let (tx, rx) = oneshot::channel();
        if self
            .tx
            .send(TaskCoordinatorMsg::GetStatus { task_id, reply: tx })
            .await
            .is_err()
        {
            return None;
        }
        rx.await.ok().flatten()
    }
}
