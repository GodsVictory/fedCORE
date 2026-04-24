# ACK EC2 Controller

AWS Controller for Kubernetes (ACK) EC2 Controller enables declarative management of Amazon EC2 resources from Kubernetes.

## Overview

The EC2 Controller allows management of EC2 resources using Kubernetes Custom Resources:
- VPCs, Subnets, Security Groups
- Instances, Elastic IPs
- Network Interfaces, Internet Gateways

## Architecture

```
ack-ec2-controller/
├── component.yaml          # Helm chart configuration
├── base/
│   └── namespace.yaml      # ack-system namespace
└── README.md               # This file
```

## References

- [ACK Documentation](https://aws-controllers-k8s.github.io/community/)
- [EC2 Controller](https://github.com/aws-controllers-k8s/ec2-controller)
