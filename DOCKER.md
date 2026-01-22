# Docker Usage Guide for Oxidowl

This guide explains how to use the Oxidowl reasoner with Docker.

## Building the Docker Image

From the project root directory:

```bash
docker build -t oxidowl:latest .
```

## Usage Patterns

### 1. Using Volume Mounts (Recommended)

Mount a local directory containing your ontology files:

```bash
# Check consistency of an ontology
docker run -v /path/to/your/ontologies:/ontologies oxidowl:latest -k /ontologies/myontology.owl

# Classify an ontology
docker run -v $(pwd)/ontologies:/ontologies oxidowl:latest -c /ontologies/greenhouse.ttl

# Multiple ontology files
docker run -v $(pwd):/ontologies oxidowl:latest -k /ontologies/onto1.owl /ontologies/onto2.owl

# With output file
docker run -v $(pwd):/ontologies oxidowl:latest -c -o /ontologies/results.txt /ontologies/myonto.owl
```

### 2. Using Standard Input

Pipe an ontology file into the container:

```bash
cat myontology.owl | docker run -i oxidowl:latest -k -

# With format specification
cat myontology.ttl | docker run -i oxidowl:latest -k --format turtle -
```

### 3. Copy Files into Container

Create a container, copy files, and run:

```bash
# Create a container from the image
docker create --name oxidowl-instance oxidowl:latest -k /ontologies/myonto.owl

# Copy your ontology into it
docker cp myontology.owl oxidowl-instance:/ontologies/myonto.owl

# Start the container
docker start -a oxidowl-instance

# Clean up
docker rm oxidowl-instance
```

### 4. Interactive Shell

Access the container for multiple operations:

```bash
# Start an interactive shell
docker run -it -v $(pwd):/ontologies --entrypoint /bin/sh oxidowl:latest

# Inside the container, you can now run multiple commands:
# oxidowl -k /ontologies/onto1.owl
# oxidowl -c /ontologies/onto2.owl
# exit
```

## Common Operations

### Consistency Checking

```bash
docker run -v $(pwd):/ontologies oxidowl:latest -k /ontologies/myonto.owl
```

### Classification

```bash
# Basic classification
docker run -v $(pwd):/ontologies oxidowl:latest -c /ontologies/myonto.owl

# With pretty printing
docker run -v $(pwd):/ontologies oxidowl:latest -c -P /ontologies/myonto.owl

# Classify object properties
docker run -v $(pwd):/ontologies oxidowl:latest -O /ontologies/myonto.owl

# Classify data properties
docker run -v $(pwd):/ontologies oxidowl:latest -D /ontologies/myonto.owl
```

### Query Operations

```bash
# Get subclasses
docker run -v $(pwd):/ontologies oxidowl:latest -s "ClassName" /ontologies/myonto.owl

# Get superclasses
docker run -v $(pwd):/ontologies oxidowl:latest -S "ClassName" /ontologies/myonto.owl

# Get equivalent classes
docker run -v $(pwd):/ontologies oxidowl:latest -e "ClassName" /ontologies/myonto.owl

# Get unsatisfiable classes
docker run -v $(pwd):/ontologies oxidowl:latest -U /ontologies/myonto.owl
```

### Using Configuration Files

```bash
# With custom config
docker run -v $(pwd):/ontologies oxidowl:latest --config /ontologies/config.json -k /ontologies/myonto.owl
```

### Verbose Output

```bash
# Verbose mode
docker run -v $(pwd):/ontologies oxidowl:latest -v -k /ontologies/myonto.owl

# Very verbose (multiple -v flags)
docker run -v $(pwd):/ontologies oxidowl:latest -vv -k /ontologies/myonto.owl
```

## Docker Compose Example

Create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  oxidowl:
    image: oxidowl:latest
    volumes:
      - ./ontologies:/ontologies
    command: -k /ontologies/myontology.owl
```

Run with:

```bash
docker-compose run oxidowl -k /ontologies/myonto.owl
```

## Advanced Usage

### Running as a Server

If using server mode (requires port mapping):

```bash
docker run -p 8080:8080 -v $(pwd):/ontologies oxidowl:latest server --host 0.0.0.0 --port 8080
```

### Resource Limits

Limit CPU and memory usage:

```bash
docker run --cpus="2.0" --memory="4g" -v $(pwd):/ontologies oxidowl:latest -c /ontologies/large-onto.owl
```

### Using Different Formats

```bash
# OWL XML
docker run -v $(pwd):/ontologies oxidowl:latest --format owx -k /ontologies/onto.owx

# Turtle
docker run -v $(pwd):/ontologies oxidowl:latest --format turtle -k /ontologies/onto.ttl

# Functional Syntax
docker run -v $(pwd):/ontologies oxidowl:latest --format ofn -k /ontologies/onto.ofn
```

## Tips and Best Practices

1. **Use volume mounts** for easier workflow - you can edit files on your host and run them immediately
2. **Use absolute paths** within the container (e.g., `/ontologies/file.owl`)
3. **Capture output** to files in the mounted volume for later analysis
4. **Use docker-compose** for repeatable workflows
5. **Set resource limits** when working with large ontologies

## Troubleshooting

### Permission Issues

If you encounter permission errors:

```bash
# Linux/macOS: Ensure your ontology files are readable
chmod -R 755 /path/to/ontologies
```

### File Not Found

Ensure the path inside the container is correct:

```bash
# Wrong:
docker run -v $(pwd):/ontologies oxidowl:latest -k myonto.owl

# Correct:
docker run -v $(pwd):/ontologies oxidowl:latest -k /ontologies/myonto.owl
```

### Large Ontologies

For very large ontologies, increase memory:

```bash
docker run --memory="8g" -v $(pwd):/ontologies oxidowl:latest -c /ontologies/large.owl
```

## Getting Help

View all available options:

```bash
docker run oxidowl:latest --help
```

## Security Notes

- The container runs as a non-root user (`oxidowl:oxidowl`) for security
- Only the `/ontologies` directory is writable by default
- Consider using read-only mounts for input files: `-v $(pwd):/ontologies:ro`
