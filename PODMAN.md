# Podman and Kubernetes Deployment Guide

This guide covers building, running, and deploying Oxidowl using Podman and Kubernetes.

## Podman

### Why Podman?

Podman is a daemonless container engine that's compatible with Docker but offers:
- Rootless containers by default
- No daemon required
- Direct integration with systemd
- Native Kubernetes YAML generation
- Compatible with Docker commands (drop-in replacement)

### Building with Podman

```bash
# Build using Containerfile
podman build -t oxidowl:latest -f Containerfile .

# Build with custom tag
podman build -t myregistry.io/oxidowl:v1.0 -f Containerfile .

# Build with build arguments
podman build --build-arg RUST_VERSION=1.88 -t oxidowl:latest -f Containerfile .
```

### Running with Podman

```bash
# Basic run
podman run oxidowl:latest --help

# With volume mount (note the :Z for SELinux)
podman run -v ./ontologies:/ontologies:Z oxidowl:latest -k /ontologies/myonto.owl

# Interactive mode
podman run -it oxidowl:latest /bin/sh

# As a detached service
podman run -d --name oxidowl-service -p 8080:8080 oxidowl:latest

# With resource limits
podman run --memory=2g --cpus=2 oxidowl:latest -k /ontologies/myonto.owl
```

### Podman Pods

Podman pods are groups of containers that share the same network namespace (similar to Kubernetes pods):

```bash
# Create a pod
podman pod create --name oxidowl-pod -p 8080:8080

# Run containers in the pod
podman run -d --pod oxidowl-pod --name oxidowl oxidowl:latest

# Check pod status
podman pod ps

# Generate Kubernetes YAML from pod
podman generate kube oxidowl-pod > oxidowl-pod.yaml

# Remove pod (stops and removes all containers)
podman pod rm -f oxidowl-pod
```

### Rootless Mode

Podman excels at rootless containers:

```bash
# Run as regular user (no sudo needed)
podman run -v ./ontologies:/ontologies:Z oxidowl:latest -k /ontologies/myonto.owl

# Check rootless status
podman info | grep -i root

# Configure rootless networking
podman network create oxidowl-net
podman run --network oxidowl-net oxidowl:latest
```

### Systemd Integration

Generate systemd units for automatic container management:

```bash
# Generate systemd unit file
podman run -d --name oxidowl oxidowl:latest
podman generate systemd --new --files --name oxidowl

# Install and enable service
mkdir -p ~/.config/systemd/user/
mv container-oxidowl.service ~/.config/systemd/user/
systemctl --user enable --now container-oxidowl.service

# Check status
systemctl --user status container-oxidowl.service
```

## Kubernetes

### Prerequisites

- Kubernetes cluster (minikube, kind, or production cluster)
- kubectl configured
- Container image built and pushed to registry (or loaded locally)

### Deploying to Kubernetes

```bash
# Create namespace and deploy all resources
kubectl apply -f k8s/

# Or deploy individually
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/pvc.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
```

### Using Local Images (Minikube/Kind)

For local development with minikube:

```bash
# Build with minikube's docker daemon
eval $(minikube docker-env)
podman build -t oxidowl:latest -f Containerfile .

# Or load image into minikube
podman save oxidowl:latest | minikube image load -
```

For kind:

```bash
# Build and load into kind
podman build -t oxidowl:latest -f Containerfile .
kind load docker-image oxidowl:latest
```

### Running Jobs

```bash
# Run a one-time processing job
kubectl apply -f k8s/job.yaml

# Check job status
kubectl get jobs -n oxidowl

# View job logs
kubectl logs -n oxidowl job/oxidowl-job

# Delete completed job
kubectl delete -f k8s/job.yaml
```

### Monitoring and Debugging

```bash
# Check all resources
kubectl get all -n oxidowl

# Describe deployment
kubectl describe deployment oxidowl -n oxidowl

# View logs
kubectl logs -n oxidowl -l app=oxidowl --tail=100 -f

# Execute commands in pod
kubectl exec -it -n oxidowl deployment/oxidowl -- /bin/sh

# Port forwarding for local access
kubectl port-forward -n oxidowl svc/oxidowl 8080:8080
```

### Scaling

```bash
# Scale deployment
kubectl scale deployment oxidowl -n oxidowl --replicas=3

# Autoscaling (requires metrics-server)
kubectl autoscale deployment oxidowl -n oxidowl --cpu-percent=80 --min=1 --max=5
```

## Podman to Kubernetes Workflow

Podman can generate Kubernetes YAML from running containers:

```bash
# 1. Run container with Podman
podman run -d --name oxidowl \
  -v ./ontologies:/ontologies:Z \
  -p 8080:8080 \
  oxidowl:latest

# 2. Generate Kubernetes YAML
podman generate kube oxidowl > oxidowl-generated.yaml

# 3. Deploy to Kubernetes
kubectl apply -f oxidowl-generated.yaml

# Alternatively, use Podman to play Kubernetes YAML
podman play kube k8s/deployment.yaml
```

## Image Registry

### Pushing to Registry

```bash
# Docker Hub
podman tag oxidowl:latest docker.io/username/oxidowl:latest
podman push docker.io/username/oxidowl:latest

# Quay.io
podman tag oxidowl:latest quay.io/username/oxidowl:latest
podman push quay.io/username/oxidowl:latest

# GitHub Container Registry
podman tag oxidowl:latest ghcr.io/username/oxidowl:latest
podman push ghcr.io/username/oxidowl:latest

# Private registry
podman tag oxidowl:latest myregistry.company.com/oxidowl:latest
podman push myregistry.company.com/oxidowl:latest
```

### Using Private Registry in Kubernetes

```bash
# Create secret for private registry
kubectl create secret docker-registry regcred \
  --docker-server=myregistry.company.com \
  --docker-username=username \
  --docker-password=password \
  -n oxidowl

# Reference in deployment
# Add to deployment.yaml spec.template.spec:
# imagePullSecrets:
# - name: regcred
```

## Best Practices

### Security

1. **Use specific image tags** instead of `latest`
2. **Run as non-root user** (already configured as UID 10001)
3. **Use read-only root filesystem** where possible
4. **Drop unnecessary capabilities**
5. **Scan images for vulnerabilities**:
   ```bash
   podman scan oxidowl:latest
   ```

### Performance

1. **Set appropriate resource limits** in Kubernetes
2. **Use persistent volumes** for data that needs to survive pod restarts
3. **Consider using multiple replicas** for high availability
4. **Use readiness and liveness probes** to ensure healthy pods

### Storage

1. **Use PersistentVolumes** for ontology files
2. **Consider using CSI drivers** for advanced storage features
3. **Backup important data** regularly
4. **Use appropriate storage classes** based on performance needs

## Troubleshooting

### Container won't start

```bash
# Check container logs
podman logs oxidowl

# Run interactively to debug
podman run -it --entrypoint /bin/sh oxidowl:latest

# Check for SELinux issues (if using volumes)
podman run -v ./ontologies:/ontologies:Z oxidowl:latest  # Note the :Z
```

### Kubernetes pod crashes

```bash
# Check pod events
kubectl describe pod -n oxidowl -l app=oxidowl

# Check logs
kubectl logs -n oxidowl -l app=oxidowl --previous

# Check resource constraints
kubectl top pod -n oxidowl
```

### Permission issues

```bash
# Check user inside container
podman run oxidowl:latest id

# Verify volume permissions
ls -la ./ontologies

# For SELinux systems
chcon -Rt svirt_sandbox_file_t ./ontologies
```

## Additional Resources

- [Podman Documentation](https://docs.podman.io/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Podman Desktop](https://podman-desktop.io/) - GUI for Podman
- [Podman Compose](https://github.com/containers/podman-compose) - Docker Compose compatibility
