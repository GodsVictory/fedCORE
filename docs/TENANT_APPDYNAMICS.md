# AppDynamics APM for Tenants

## Overview

The fedcore platform provides two levels of AppDynamics observability:

1. **Platform-Provided (Automatic)**: Istio service mesh traces
   - Network-level visibility
   - Service-to-service communication
   - Zero configuration required when Istio is enabled

2. **Tenant-Provided (This Guide)**: AppDynamics APM agents
   - Code-level visibility
   - Method tracing, SQL queries, exceptions
   - Requires adding agents to your containers

## Prerequisites

- AppDynamics Controller access (open GitHub issue for access)
- Your application name in AppDynamics
- Language-specific agent download

**Platform-Provided:**
- ✅ AppDynamics controller configuration (cluster-wide secret)
- ✅ NetworkPolicy allowing egress to AppDynamics controller
- ✅ RBAC permissions to access shared configuration

## Quick Start by Language

### Java Applications

**1. Add agent to your container:**

```dockerfile
FROM openjdk:11
WORKDIR /app

# Download AppDynamics Java agent
# Option A: From shared location (recommended)
COPY --from=appdynamics-java-agent:latest /opt/appdynamics /opt/appdynamics

# Option B: Download directly (for reference)
# RUN curl -L -o /opt/appdynamics-java-agent.zip \
#     https://download.appdynamics.com/download/... && \
#     unzip /opt/appdynamics-java-agent.zip -d /opt/

# Your application
COPY target/myapp.jar /app/app.jar

# Configure Java agent
ENV JAVA_TOOL_OPTIONS="-javaagent:/opt/appdynamics/javaagent.jar"

# AppDynamics configuration
ENV APPDYNAMICS_AGENT_APPLICATION_NAME="MyApp"
ENV APPDYNAMICS_AGENT_TIER_NAME="${HOSTNAME}"
ENV APPDYNAMICS_AGENT_NODE_NAME="${HOSTNAME}"

# Reference shared controller config (platform-provided)
ENV APPDYNAMICS_CONTROLLER_HOST_NAME="your-tenant.saas.appdynamics.com"
ENV APPDYNAMICS_CONTROLLER_PORT="443"
ENV APPDYNAMICS_CONTROLLER_SSL_ENABLED="true"
ENV APPDYNAMICS_AGENT_ACCOUNT_NAME="your-account"
ENV APPDYNAMICS_AGENT_ACCOUNT_ACCESS_KEY="your-access-key"

CMD ["java", "-jar", "/app/app.jar"]
```

**2. Or use Kubernetes secrets:**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: myapp
  namespace: acme-frontend
spec:
  containers:
  - name: app
    image: myapp:1.0
    env:
    # Reference platform-provided config
    - name: APPDYNAMICS_CONTROLLER_HOST_NAME
      valueFrom:
        secretKeyRef:
          name: appdynamics-controller-config
          namespace: appdynamics-config
          key: controller-host
    - name: APPDYNAMICS_CONTROLLER_PORT
      valueFrom:
        secretKeyRef:
          name: appdynamics-controller-config
          namespace: appdynamics-config
          key: controller-port
    # Your app-specific config
    - name: APPDYNAMICS_AGENT_APPLICATION_NAME
      value: "ACME-Frontend"
    - name: APPDYNAMICS_AGENT_TIER_NAME
      value: "web"
```

### Node.js Applications

```dockerfile
FROM node:18
WORKDIR /app

# Install AppDynamics agent
RUN npm install appdynamics@latest

# Your application
COPY package*.json ./
RUN npm install
COPY . .

# Configure agent (via environment or appdynamics.js)
ENV APPDYNAMICS_AGENT_APPLICATION_NAME="MyNodeApp"
ENV APPDYNAMICS_CONTROLLER_HOST_NAME="your-tenant.saas.appdynamics.com"

# Require AppDynamics at startup
CMD ["node", "-r", "appdynamics", "server.js"]
```

**Or in code (appdynamics.js):**

```javascript
// appdynamics.js
require('appdynamics').profile({
  controllerHostName: process.env.APPDYNAMICS_CONTROLLER_HOST_NAME,
  controllerPort: 443,
  controllerSslEnabled: true,
  accountName: process.env.APPDYNAMICS_AGENT_ACCOUNT_NAME,
  accountAccessKey: process.env.APPDYNAMICS_AGENT_ACCOUNT_ACCESS_KEY,
  applicationName: 'MyNodeApp',
  tierName: 'api',
  nodeName: process.env.HOSTNAME
});

// Then require your app
require('./server.js');
```

### Python Applications

```dockerfile
FROM python:3.11
WORKDIR /app

# Install AppDynamics agent
RUN pip install appdynamics

# Your application
COPY requirements.txt .
RUN pip install -r requirements.txt
COPY . .

# Configure agent
ENV APPDYNAMICS_AGENT_APPLICATION_NAME="MyPythonApp"
ENV APPDYNAMICS_CONTROLLER_HOST_NAME="your-tenant.saas.appdynamics.com"

# Start with AppDynamics
CMD ["pyagent", "run", "--", "python", "app.py"]
```

### .NET Applications

```dockerfile
FROM mcr.microsoft.com/dotnet/aspnet:7.0
WORKDIR /app

# Copy AppDynamics .NET agent
COPY --from=appdynamics-dotnet-agent:latest /opt/appdynamics /opt/appdynamics

# Your application
COPY bin/Release/net7.0/publish/ .

# Configure AppDynamics
ENV APPDYNAMICS_AGENT_APPLICATION_NAME="MyDotNetApp"
ENV APPDYNAMICS_CONTROLLER_HOST_NAME="your-tenant.saas.appdynamics.com"
ENV CORECLR_ENABLE_PROFILING=1
ENV CORECLR_PROFILER={57e1aa68-2229-41aa-9931-a6e93bbc64d8}
ENV CORECLR_PROFILER_PATH=/opt/appdynamics/libappdprofiler.so

ENTRYPOINT ["dotnet", "MyApp.dll"]
```

## Using with Istio Service Mesh

If your tenant has Istio enabled, you get **both** levels of observability:

```yaml
# Enable Istio in your TenantOnboarding
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  settings:
    istio:
      enabled: true
      strictMTLS: true
```

**Result in AppDynamics:**
- **Network-level traces** from Istio (automatic)
- **Code-level traces** from APM agents (if you add them)
- **Correlated view** showing complete request journey

## Configuration Best Practices

### 1. Use Platform-Provided Controller Config

Reference the shared secret instead of hardcoding:

```yaml
env:
  - name: APPDYNAMICS_CONTROLLER_HOST_NAME
    valueFrom:
      secretKeyRef:
        name: appdynamics-controller-config
        namespace: appdynamics-config
        key: controller-host
```

### 2. Use Meaningful Names

**Application Name:** Group related services
```
APPDYNAMICS_AGENT_APPLICATION_NAME="ACME-Platform"
```

**Tier Name:** Service type
```
APPDYNAMICS_AGENT_TIER_NAME="frontend"  # or "api", "worker", etc.
```

**Node Name:** Use pod hostname
```
APPDYNAMICS_AGENT_NODE_NAME="${HOSTNAME}"
```

### 3. Tag with Tenant Context

Add custom properties for multi-tenancy:

```yaml
env:
  - name: APPDYNAMICS_AGENT_APPLICATION_NAME
    value: "ACME-Platform"
  - name: APPDYNAMICS_AGENT_TIER_NAME
    value: "frontend"
  # Custom properties
  - name: APPDYNAMICS_CONTROLLER_INFO_EXTRA
    value: "tenant=acme,cluster=fedcore-prod-use1,environment=prod"
```

### 4. Resource Overhead

Plan for agent overhead:

| Language | CPU Overhead | Memory Overhead |
|----------|-------------|-----------------|
| Java | 5-10% | 50-100Mi |
| Node.js | 5-10% | 30-50Mi |
| Python | 5-10% | 30-50Mi |
| .NET | 5-10% | 50-80Mi |

**Update your resource requests:**

```yaml
resources:
  requests:
    cpu: 550m      # 500m app + 50m agent
    memory: 562Mi  # 512Mi app + 50Mi agent
  limits:
    cpu: 1100m
    memory: 1124Mi
```

## Network Connectivity

### Outbound Access Required

Your pods need HTTPS (443) access to:
- **AppDynamics SaaS**: `*.saas.appdynamics.com`
- **AppDynamics On-Prem**: Your controller hostname

**Platform configuration:**
The platform already allows internet egress for tenants. No additional NetworkPolicy changes needed.

**Verify connectivity:**

```bash
# From your pod
kubectl exec -it <pod-name> -n <namespace> -- \
  curl -v https://your-tenant.saas.appdynamics.com
```

## Troubleshooting

### Agent Not Reporting

**1. Check agent logs:**

```bash
# Java
kubectl logs <pod-name> -n <namespace> | grep -i appdynamics

# Node.js
kubectl logs <pod-name> -n <namespace> | grep -i "AppDynamics Agent"

# Python
kubectl logs <pod-name> -n <namespace> | grep -i pyagent
```

**2. Verify controller connectivity:**

```bash
kubectl exec -it <pod-name> -n <namespace> -- \
  curl -v https://<controller-host>:<port>/controller/
```

**3. Check configuration:**

```bash
# View environment variables
kubectl exec <pod-name> -n <namespace> -- env | grep APPDYNAMICS
```

**4. Common issues:**

- **Controller unreachable**: Check NetworkPolicy, firewall rules
- **Invalid access key**: Verify credentials with AppDynamics team
- **Application not appearing**: Check application name spelling
- **High memory usage**: Increase pod memory limits

### Agent Causing Performance Issues

**Reduce sampling:**

```yaml
env:
  # Sample 10% of transactions
  - name: APPDYNAMICS_AGENT_PROXY_CTRL_SAMPLING_PERCENTAGE
    value: "10"
```

**Disable features:**

```yaml
env:
  # Disable expensive features
  - name: APPDYNAMICS_AGENT_ENABLE_BUSINESS_TRANSACTIONS
    value: "true"
  - name: APPDYNAMICS_AGENT_ENABLE_SQL_CAPTURE
    value: "false"  # Disable if causing overhead
```

## Viewing Data in AppDynamics

### Access the Controller

**URL**: https://your-tenant.saas.appdynamics.com

**Navigation:**
1. **Applications** → Your application name
2. **Transaction Snapshots** → View individual requests
3. **Flow Map** → Service dependency visualization
4. **Metrics** → Request rate, error rate, latency

### Filter by Tenant

Use custom properties to filter:

```
tenant = "acme"
cluster = "fedcore-prod-use1"
environment = "prod"
```

### Correlated with Istio Traces

If Istio is enabled, you'll see:
- **Network-level spans** from Istio Envoy
- **Code-level spans** from APM agent
- **Complete request flow** across both

## Cost Optimization

### License Consumption

AppDynamics licenses based on:
- **APM units** per agent hour
- **Transaction volume**

**Tips to reduce cost:**

1. **Sample strategically:**
   - 100% sampling for critical services
   - 10-20% for high-volume APIs

2. **Use tiered approach:**
   - Production: Full instrumentation
   - Staging: Selective instrumentation
   - Dev: Minimal or no agents

3. **Disable in non-production:**
   ```yaml
   # Only enable in production
   - name: APPDYNAMICS_AGENT_ENABLED
     value: "{{ .Values.environment == 'prod' ? 'true' : 'false' }}"
   ```

## Support

### Platform Team
- Network connectivity issues
- Shared configuration problems
- Istio integration questions
- Contact: Open GitHub issue with label "appdynamics"

### AppDynamics Team
- Agent installation
- Controller access
- License questions
- Custom dashboards
- Contact: Open GitHub issue with label "appdynamics-access"

## Additional Resources

- [AppDynamics Java Agent Docs](https://docs.appdynamics.com/display/latest/Java+Agent)
- [AppDynamics Node.js Agent Docs](https://docs.appdynamics.com/display/latest/Node.js+Agent)
- [AppDynamics Python Agent Docs](https://docs.appdynamics.com/display/latest/Python+Agent)
- [AppDynamics .NET Agent Docs](https://docs.appdynamics.com/display/latest/.NET+Agent)
- [Istio Integration](../platform/components/istio/APPDYNAMICS_INTEGRATION.md)

## Examples

### Complete Java Example with Helm

```yaml
# values.yaml
appdynamics:
  enabled: true
  applicationName: "ACME-Platform"
  tierName: "api"

# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-server
spec:
  template:
    spec:
      containers:
      - name: app
        image: acme/api-server:1.0
        env:
        {{- if .Values.appdynamics.enabled }}
        - name: JAVA_TOOL_OPTIONS
          value: "-javaagent:/opt/appdynamics/javaagent.jar"
        - name: APPDYNAMICS_AGENT_APPLICATION_NAME
          value: {{ .Values.appdynamics.applicationName }}
        - name: APPDYNAMICS_AGENT_TIER_NAME
          value: {{ .Values.appdynamics.tierName }}
        - name: APPDYNAMICS_AGENT_NODE_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: APPDYNAMICS_CONTROLLER_HOST_NAME
          valueFrom:
            secretKeyRef:
              name: appdynamics-controller-config
              namespace: appdynamics-config
              key: controller-host
        {{- end }}
        resources:
          requests:
            cpu: {{ .Values.appdynamics.enabled ? "550m" : "500m" }}
            memory: {{ .Values.appdynamics.enabled ? "562Mi" : "512Mi" }}
```

### GitOps Workflow

```bash
# 1. Add agent to your Dockerfile
git add Dockerfile
git commit -m "Add AppDynamics Java agent"

# 2. Update Helm values
git add values.yaml
git commit -m "Enable AppDynamics for production"

# 3. Push to trigger deployment
git push origin main

# 4. Verify in cluster
kubectl get pods -n acme-api
kubectl logs <pod-name> -n acme-api | grep -i appdynamics

# 5. Check AppDynamics Controller
# Navigate to Applications → ACME-Platform
```

## FAQ

**Q: Do I need AppDynamics if I have Istio?**
A: Istio provides network-level visibility. AppDynamics agents provide code-level visibility (method calls, SQL queries, exceptions). Both together give complete observability.

**Q: Can I use AppDynamics without Istio?**
A: Yes! AppDynamics agents work independently of Istio. Istio is optional.

**Q: What's the performance impact?**
A: Typically 5-10% CPU and 30-100Mi memory depending on language and configuration.

**Q: How do I get an AppDynamics license?**
A: Open a GitHub issue with label "appdynamics-license" for license allocation.

**Q: Can I test locally?**
A: Yes, AppDynamics agents work in local development. Use a dev/test controller or AppDynamics Lite.

**Q: What about serverless/Lambda?**
A: AppDynamics supports AWS Lambda with a different agent. Open a GitHub issue for guidance.

---

## Navigation

[← Previous: Helm Charts](HELM_CHARTS.md)

**Handbook Progress:** Page 35 of 35 - 🎉 Handbook Complete! | **Level 7:** Advanced Features

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
