//! Performance prediction models for ML-driven query optimization
//!
//! This module implements various machine learning models for predicting
//! query execution performance based on extracted features.

use super::optimizer::{AccuracyMetrics, PerformancePredictionModel, QueryPerformanceDataPoint};
use std::collections::VecDeque;

/// Simple linear regression model for performance prediction
#[derive(Debug)]
pub struct LinearRegressionModel {
    /// Model coefficients for execution time prediction
    execution_time_coefficients: Vec<f64>,

    /// Model coefficients for memory usage prediction
    memory_usage_coefficients: Vec<f64>,

    /// Bias terms
    execution_time_bias: f64,
    memory_usage_bias: f64,

    /// Training data history (for continuous learning)
    training_history: VecDeque<QueryPerformanceDataPoint>,

    /// Maximum training data to keep
    max_history_size: usize,

    /// Model accuracy metrics
    accuracy_metrics: AccuracyMetrics,
}

/// Neural network-based performance prediction model
#[derive(Debug)]
pub struct NeuralNetworkModel {
    /// Network layers (simplified representation)
    layers: Vec<Layer>,

    /// Learning rate for gradient descent
    learning_rate: f64,

    /// Training data history
    training_history: VecDeque<QueryPerformanceDataPoint>,

    /// Maximum training data to keep
    max_history_size: usize,

    /// Model accuracy metrics
    accuracy_metrics: AccuracyMetrics,
}

/// Ensemble model combining multiple prediction approaches
#[derive(Debug)]
pub struct EnsembleModel {
    /// Component models
    models: Vec<Box<dyn PerformancePredictionModel>>,

    /// Weights for each model in the ensemble
    model_weights: Vec<f64>,

    /// Model accuracy metrics
    accuracy_metrics: AccuracyMetrics,
}

/// Neural network layer (simplified)
#[derive(Debug)]
pub struct Layer {
    /// Weights matrix (flattened)
    weights: Vec<f64>,

    /// Bias vector
    biases: Vec<f64>,

    /// Number of inputs
    input_size: usize,

    /// Number of outputs
    output_size: usize,

    /// Activation function
    activation: ActivationFunction,
}

/// Activation functions for neural network
#[derive(Debug, Clone)]
pub enum ActivationFunction {
    ReLU,
    Sigmoid,
    Tanh,
    Linear,
}

/// Gradients for a single layer during backpropagation
#[derive(Debug)]
struct LayerGradients {
    weight_gradients: Vec<f64>,
    bias_gradients: Vec<f64>,
}

impl LinearRegressionModel {
    /// Create a new linear regression model
    #[must_use] 
    pub fn new(feature_count: usize) -> Self {
        Self {
            execution_time_coefficients: vec![0.1; feature_count], // Initialize with small random values
            memory_usage_coefficients: vec![0.1; feature_count],
            execution_time_bias: 0.0,
            memory_usage_bias: 0.0,
            training_history: VecDeque::new(),
            max_history_size: 1000,
            accuracy_metrics: AccuracyMetrics::default(),
        }
    }

    /// Perform linear regression training using least squares
    fn train_linear_regression(&mut self, data: &[QueryPerformanceDataPoint]) {
        if data.is_empty() {
            return;
        }

        let n = data.len();
        let feature_count = data[0].query_features.len();

        // Simple gradient descent implementation
        let learning_rate = 0.01;
        let iterations = 100;

        for _ in 0..iterations {
            let mut exec_time_gradient = vec![0.0; feature_count];
            let mut exec_time_bias_gradient = 0.0;
            let mut memory_gradient = vec![0.0; feature_count];
            let mut memory_bias_gradient = 0.0;

            // Calculate gradients
            for point in data {
                let exec_prediction = self.predict_execution_time(&point.query_features);
                let memory_prediction = self.predict_memory_usage(&point.query_features);

                let exec_error = exec_prediction - point.execution_time;
                let memory_error = memory_prediction - point.memory_usage;

                // Update gradients for execution time
                for (i, &feature) in point.query_features.iter().enumerate() {
                    exec_time_gradient[i] += exec_error * feature / n as f64;
                }
                exec_time_bias_gradient += exec_error / n as f64;

                // Update gradients for memory usage
                for (i, &feature) in point.query_features.iter().enumerate() {
                    memory_gradient[i] += memory_error * feature / n as f64;
                }
                memory_bias_gradient += memory_error / n as f64;
            }

            // Apply gradients
            for i in 0..feature_count {
                self.execution_time_coefficients[i] -= learning_rate * exec_time_gradient[i];
                self.memory_usage_coefficients[i] -= learning_rate * memory_gradient[i];
            }
            self.execution_time_bias -= learning_rate * exec_time_bias_gradient;
            self.memory_usage_bias -= learning_rate * memory_bias_gradient;
        }

        // Update accuracy metrics
        self.update_accuracy_metrics(data);
    }

    /// Update accuracy metrics based on current model performance
    fn update_accuracy_metrics(&mut self, data: &[QueryPerformanceDataPoint]) {
        if data.is_empty() {
            return;
        }

        let mut exec_errors = Vec::new();
        let mut memory_errors = Vec::new();
        let mut exec_actuals = Vec::new();
        let mut memory_actuals = Vec::new();

        for point in data {
            let exec_prediction = self.predict_execution_time(&point.query_features);
            let memory_prediction = self.predict_memory_usage(&point.query_features);

            exec_errors.push((exec_prediction - point.execution_time).abs());
            memory_errors.push((memory_prediction - point.memory_usage).abs());
            exec_actuals.push(point.execution_time);
            memory_actuals.push(point.memory_usage);
        }

        // Calculate mean absolute error
        let exec_mae = exec_errors.iter().sum::<f64>() / exec_errors.len() as f64;
        let memory_mae = memory_errors.iter().sum::<f64>() / memory_errors.len() as f64;

        // Calculate root mean square error
        let exec_mse = exec_errors.iter().map(|e| e * e).sum::<f64>() / exec_errors.len() as f64;
        let exec_rmse = exec_mse.sqrt();

        // Simple correlation coefficient calculation (simplified)
        let correlation = self.calculate_correlation(&exec_actuals, data);

        self.accuracy_metrics = AccuracyMetrics {
            mean_absolute_error: (exec_mae + memory_mae) / 2.0,
            root_mean_square_error: exec_rmse,
            correlation_coefficient: correlation,
            prediction_count: data.len(),
        };
    }

    /// Calculate correlation coefficient (simplified)
    fn calculate_correlation(&self, actuals: &[f64], data: &[QueryPerformanceDataPoint]) -> f64 {
        if actuals.len() < 2 {
            return 0.0;
        }

        let predictions: Vec<f64> = data
            .iter()
            .map(|point| self.predict_execution_time(&point.query_features))
            .collect();

        let actual_mean = actuals.iter().sum::<f64>() / actuals.len() as f64;
        let pred_mean = predictions.iter().sum::<f64>() / predictions.len() as f64;

        let numerator: f64 = actuals
            .iter()
            .zip(predictions.iter())
            .map(|(a, p)| (a - actual_mean) * (p - pred_mean))
            .sum();

        let actual_var: f64 = actuals.iter().map(|a| (a - actual_mean).powi(2)).sum();

        let pred_var: f64 = predictions.iter().map(|p| (p - pred_mean).powi(2)).sum();

        let denominator = (actual_var * pred_var).sqrt();

        if denominator > 0.0 {
            numerator / denominator
        } else {
            0.0
        }
    }
}

impl PerformancePredictionModel for LinearRegressionModel {
    fn predict_execution_time(&self, features: &[f64]) -> f64 {
        let prediction = features
            .iter()
            .zip(self.execution_time_coefficients.iter())
            .map(|(f, c)| f * c)
            .sum::<f64>()
            + self.execution_time_bias;

        // Ensure non-negative prediction
        prediction.max(0.01)
    }

    fn predict_memory_usage(&self, features: &[f64]) -> f64 {
        let prediction = features
            .iter()
            .zip(self.memory_usage_coefficients.iter())
            .map(|(f, c)| f * c)
            .sum::<f64>()
            + self.memory_usage_bias;

        // Ensure non-negative prediction
        prediction.max(1024.0) // At least 1KB
    }

    fn train(&mut self, training_data: &[QueryPerformanceDataPoint]) {
        // Add new training data to history
        for point in training_data {
            self.training_history.push_back(point.clone());

            // Keep only recent data
            if self.training_history.len() > self.max_history_size {
                self.training_history.pop_front();
            }
        }

        // Train the model on all historical data
        let data: Vec<_> = self.training_history.iter().cloned().collect();
        self.train_linear_regression(&data);
    }

    fn get_accuracy(&self) -> AccuracyMetrics {
        self.accuracy_metrics.clone()
    }
}

impl NeuralNetworkModel {
    /// Create a new neural network model
    #[must_use] 
    pub fn new(feature_count: usize, hidden_sizes: Vec<usize>) -> Self {
        let mut layers = Vec::new();
        let mut input_size = feature_count;

        // Create hidden layers
        for &hidden_size in &hidden_sizes {
            layers.push(Layer::new(
                input_size,
                hidden_size,
                ActivationFunction::ReLU,
            ));
            input_size = hidden_size;
        }

        // Create output layer (2 outputs: execution_time, memory_usage)
        layers.push(Layer::new(input_size, 2, ActivationFunction::Linear));

        Self {
            layers,
            learning_rate: 0.01,
            training_history: VecDeque::new(),
            max_history_size: 1000,
            accuracy_metrics: AccuracyMetrics::default(),
        }
    }

    /// Forward pass through the network
    fn forward(&self, input: &[f64]) -> Vec<f64> {
        let mut current_output = input.to_vec();

        for layer in &self.layers {
            current_output = layer.forward(&current_output);
        }

        current_output
    }

    /// Train network using backpropagation with proper gradient computation
    fn train_network(&mut self, data: &[QueryPerformanceDataPoint]) {
        if data.is_empty() {
            return;
        }

        let epochs = 100;
        let batch_size = 32.min(data.len());

        for _ in 0..epochs {
            // Mini-batch gradient descent
            for batch_start in (0..data.len()).step_by(batch_size) {
                let batch_end = (batch_start + batch_size).min(data.len());
                let batch = &data[batch_start..batch_end];

                // Accumulate gradients for batch
                let mut layer_gradients: Vec<LayerGradients> = self
                    .layers
                    .iter()
                    .map(|layer| LayerGradients {
                        weight_gradients: vec![0.0; layer.weights.len()],
                        bias_gradients: vec![0.0; layer.biases.len()],
                    })
                    .collect();

                for point in batch {
                    // Forward pass with activation storage
                    let activations = self.forward_with_activations(&point.query_features);

                    // Compute output error
                    let Some(output) = activations.last() else {
                        log::warn!("No activation layers in neural network");
                        continue;
                    };
                    let target = [point.execution_time, point.memory_usage];
                    let mut delta: Vec<f64> = output
                        .iter()
                        .zip(target.iter())
                        .map(|(o, t)| o - t)
                        .collect();

                    // Backward pass through layers
                    for i in (0..self.layers.len()).rev() {
                        let layer = &self.layers[i];
                        let input = if i == 0 {
                            &point.query_features
                        } else {
                            &activations[i - 1]
                        };

                        // Compute gradients for this layer
                        for (j, &d) in delta.iter().enumerate().take(layer.output_size) {
                            for (k, &inp) in input.iter().enumerate().take(layer.input_size) {
                                let weight_idx = j * layer.input_size + k;
                                layer_gradients[i].weight_gradients[weight_idx] += d * inp;
                            }
                            layer_gradients[i].bias_gradients[j] += d;
                        }

                        // Propagate error to previous layer
                        if i > 0 {
                            let mut new_delta = vec![0.0; layer.input_size];
                            for (j, nd) in new_delta.iter_mut().enumerate() {
                                for (k, &d) in delta.iter().enumerate().take(layer.output_size) {
                                    let weight_idx = k * layer.input_size + j;
                                    *nd += d * layer.weights[weight_idx];
                                }
                                // Apply activation derivative
                                *nd *= self.activation_derivative(
                                    activations[i - 1][j],
                                    &self.layers[i - 1].activation,
                                );
                            }
                            delta = new_delta;
                        }
                    }
                }

                // Update weights using accumulated gradients
                let batch_size_f64 = batch.len() as f64;
                for (i, gradients) in layer_gradients.iter().enumerate() {
                    let layer = &mut self.layers[i];
                    for j in 0..layer.weights.len() {
                        layer.weights[j] -=
                            self.learning_rate * gradients.weight_gradients[j] / batch_size_f64;
                    }
                    for j in 0..layer.biases.len() {
                        layer.biases[j] -=
                            self.learning_rate * gradients.bias_gradients[j] / batch_size_f64;
                    }
                }
            }
        }

        self.update_accuracy_metrics(data);
    }

    /// Forward pass that stores activations for backpropagation
    fn forward_with_activations(&self, input: &[f64]) -> Vec<Vec<f64>> {
        let mut activations = Vec::new();
        let mut current_output = input.to_vec();

        for layer in &self.layers {
            current_output = layer.forward(&current_output);
            activations.push(current_output.clone());
        }

        activations
    }

    /// Compute derivative of activation function
    fn activation_derivative(&self, x: f64, activation: &ActivationFunction) -> f64 {
        match activation {
            ActivationFunction::ReLU => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            ActivationFunction::Sigmoid => {
                let sig = 1.0 / (1.0 + (-x).exp());
                sig * (1.0 - sig)
            }
            ActivationFunction::Tanh => {
                let tanh = x.tanh();
                1.0 - tanh * tanh
            }
            ActivationFunction::Linear => 1.0,
        }
    }

    /// Update accuracy metrics
    fn update_accuracy_metrics(&mut self, data: &[QueryPerformanceDataPoint]) {
        if data.is_empty() {
            return;
        }

        let mut errors = Vec::new();

        for point in data {
            let prediction = self.forward(&point.query_features);
            let exec_error = (prediction[0] - point.execution_time).abs();
            let memory_error = (prediction[1] - point.memory_usage).abs();
            errors.push((exec_error + memory_error) / 2.0);
        }

        let mae = errors.iter().sum::<f64>() / errors.len() as f64;
        let mse = errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64;

        // Calculate correlation coefficient between predictions and actual values
        let mean_actual = data.iter().map(|p| p.execution_time).sum::<f64>() / data.len() as f64;
        let mean_pred = data.iter().map(|p| {
            let features = &p.query_features;
            self.predict_execution_time(features)
        }).sum::<f64>() / data.len() as f64;
        
        let mut covariance = 0.0;
        let mut var_actual = 0.0;
        let mut var_pred = 0.0;
        
        for point in data {
            let features = &point.query_features;
            let pred = self.predict_execution_time(features);
            covariance += (point.execution_time - mean_actual) * (pred - mean_pred);
            var_actual += (point.execution_time - mean_actual).powi(2);
            var_pred += (pred - mean_pred).powi(2);
        }
        
        let correlation = if var_actual > 0.0 && var_pred > 0.0 {
            covariance / (var_actual.sqrt() * var_pred.sqrt())
        } else {
            0.0
        };
        
        self.accuracy_metrics = AccuracyMetrics {
            mean_absolute_error: mae,
            root_mean_square_error: mse.sqrt(),
            correlation_coefficient: correlation.abs(),
            prediction_count: data.len(),
        };
    }
}

impl PerformancePredictionModel for NeuralNetworkModel {
    fn predict_execution_time(&self, features: &[f64]) -> f64 {
        let output = self.forward(features);
        output[0].max(0.01)
    }

    fn predict_memory_usage(&self, features: &[f64]) -> f64 {
        let output = self.forward(features);
        output[1].max(1024.0)
    }

    fn train(&mut self, training_data: &[QueryPerformanceDataPoint]) {
        // Add to history
        for point in training_data {
            self.training_history.push_back(point.clone());
            if self.training_history.len() > self.max_history_size {
                self.training_history.pop_front();
            }
        }

        // Train on historical data
        let data: Vec<_> = self.training_history.iter().cloned().collect();
        self.train_network(&data);
    }

    fn get_accuracy(&self) -> AccuracyMetrics {
        self.accuracy_metrics.clone()
    }
}

impl Layer {
    /// Create a new layer
    fn new(input_size: usize, output_size: usize, activation: ActivationFunction) -> Self {
        // Initialize weights with small random values
        let weights = vec![0.1; input_size * output_size];
        let biases = vec![0.0; output_size];

        Self {
            weights,
            biases,
            input_size,
            output_size,
            activation,
        }
    }

    /// Forward pass through the layer
    fn forward(&self, input: &[f64]) -> Vec<f64> {
        let mut output = vec![0.0; self.output_size];

        // Matrix multiplication: output = weights * input + bias
        for (i, out) in output.iter_mut().enumerate() {
            for (j, &inp) in input.iter().enumerate() {
                *out += self.weights[i * self.input_size + j] * inp;
            }
            *out += self.biases[i];
        }

        // Apply activation function
        for value in &mut output {
            *value = self.activation.apply(*value);
        }

        output
    }
}

impl ActivationFunction {
    /// Apply the activation function
    fn apply(&self, x: f64) -> f64 {
        match self {
            ActivationFunction::ReLU => x.max(0.0),
            ActivationFunction::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            ActivationFunction::Tanh => x.tanh(),
            ActivationFunction::Linear => x,
        }
    }
}

impl EnsembleModel {
    /// Create a new ensemble model
    #[must_use] 
    pub fn new(models: Vec<Box<dyn PerformancePredictionModel>>) -> Self {
        let model_count = models.len();
        let equal_weights = vec![1.0 / model_count as f64; model_count];

        Self {
            models,
            model_weights: equal_weights,
            accuracy_metrics: AccuracyMetrics::default(),
        }
    }

    /// Update model weights based on individual model performance
    fn update_weights(&mut self) {
        let accuracies: Vec<f64> = self
            .models
            .iter()
            .map(|model| {
                let metrics = model.get_accuracy();
                // Use inverse of MAE as a measure of model quality
                if metrics.mean_absolute_error > 0.0 {
                    1.0 / metrics.mean_absolute_error
                } else {
                    1.0
                }
            })
            .collect();

        let total_accuracy: f64 = accuracies.iter().sum();

        if total_accuracy > 0.0 {
            self.model_weights = accuracies.iter().map(|acc| acc / total_accuracy).collect();
        }
    }
}

impl PerformancePredictionModel for EnsembleModel {
    fn predict_execution_time(&self, features: &[f64]) -> f64 {
        self.models
            .iter()
            .zip(self.model_weights.iter())
            .map(|(model, weight)| model.predict_execution_time(features) * weight)
            .sum()
    }

    fn predict_memory_usage(&self, features: &[f64]) -> f64 {
        self.models
            .iter()
            .zip(self.model_weights.iter())
            .map(|(model, weight)| model.predict_memory_usage(features) * weight)
            .sum()
    }

    fn train(&mut self, training_data: &[QueryPerformanceDataPoint]) {
        // Train all component models
        for model in &mut self.models {
            model.train(training_data);
        }

        // Update ensemble weights based on individual model performance
        self.update_weights();

        // Update ensemble accuracy metrics
        if !training_data.is_empty() {
            let mut errors = Vec::new();

            for point in training_data {
                let exec_pred = self.predict_execution_time(&point.query_features);
                let memory_pred = self.predict_memory_usage(&point.query_features);
                let exec_error = (exec_pred - point.execution_time).abs();
                let memory_error = (memory_pred - point.memory_usage).abs();
                errors.push((exec_error + memory_error) / 2.0);
            }

            let mae = errors.iter().sum::<f64>() / errors.len() as f64;
            let mse = errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64;

            // Calculate correlation coefficient between predictions and actual execution times
            let mean_actual_exec = training_data.iter().map(|p| p.execution_time).sum::<f64>() / training_data.len() as f64;
            let mean_pred_exec = training_data.iter().map(|p| {
                self.predict_execution_time(&p.query_features)
            }).sum::<f64>() / training_data.len() as f64;
            
            let mut covariance = 0.0;
            let mut var_actual = 0.0;
            let mut var_pred = 0.0;
            
            for point in training_data {
                let pred_exec = self.predict_execution_time(&point.query_features);
                covariance += (point.execution_time - mean_actual_exec) * (pred_exec - mean_pred_exec);
                var_actual += (point.execution_time - mean_actual_exec).powi(2);
                var_pred += (pred_exec - mean_pred_exec).powi(2);
            }
            
            let correlation = if var_actual > 0.0 && var_pred > 0.0 {
                covariance / (var_actual.sqrt() * var_pred.sqrt())
            } else {
                0.0
            };
            
            self.accuracy_metrics = AccuracyMetrics {
                mean_absolute_error: mae,
                root_mean_square_error: mse.sqrt(),
                correlation_coefficient: correlation.abs().min(1.0),
                prediction_count: training_data.len(),
            };
        }
    }

    fn get_accuracy(&self) -> AccuracyMetrics {
        self.accuracy_metrics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_linear_regression_model() {
        let mut model = LinearRegressionModel::new(3);

        // Create sample training data
        let training_data = vec![
            QueryPerformanceDataPoint {
                query_features: vec![1.0, 2.0, 3.0],
                execution_time: 100.0,
                memory_usage: 1024.0,
                result_size: 10,
                timestamp: Instant::now(),
            },
            QueryPerformanceDataPoint {
                query_features: vec![2.0, 3.0, 4.0],
                execution_time: 200.0,
                memory_usage: 2048.0,
                result_size: 20,
                timestamp: Instant::now(),
            },
        ];

        model.train(&training_data);

        // Test prediction
        let prediction = model.predict_execution_time(&[1.5, 2.5, 3.5]);
        assert!(prediction > 0.0);

        let memory_prediction = model.predict_memory_usage(&[1.5, 2.5, 3.5]);
        assert!(memory_prediction >= 1024.0);
    }

    #[test]
    fn test_neural_network_model() {
        let mut model = NeuralNetworkModel::new(3, vec![5, 3]);

        // Test forward pass
        let prediction = model.forward(&[1.0, 2.0, 3.0]);
        assert_eq!(prediction.len(), 2); // execution_time, memory_usage

        // Test training
        let training_data = vec![QueryPerformanceDataPoint {
            query_features: vec![1.0, 2.0, 3.0],
            execution_time: 100.0,
            memory_usage: 1024.0,
            result_size: 10,
            timestamp: Instant::now(),
        }];

        model.train(&training_data);

        // Test prediction after training
        let exec_pred = model.predict_execution_time(&[1.0, 2.0, 3.0]);
        let memory_pred = model.predict_memory_usage(&[1.0, 2.0, 3.0]);

        assert!(exec_pred > 0.0);
        assert!(memory_pred >= 1024.0);
    }

    #[test]
    fn test_activation_functions() {
        assert_eq!(ActivationFunction::ReLU.apply(-1.0), 0.0);
        assert_eq!(ActivationFunction::ReLU.apply(1.0), 1.0);

        assert!(ActivationFunction::Sigmoid.apply(0.0) > 0.0);
        assert!(ActivationFunction::Sigmoid.apply(0.0) < 1.0);

        assert_eq!(ActivationFunction::Linear.apply(2.0), 2.0);
    }
}
