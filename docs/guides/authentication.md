# Authentication Guide

Secure your LLM Memory Graph deployment.

## TLS/SSL Configuration

### Server-side TLS

```bash
# Generate certificates
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout server-key.pem \
  -out server-cert.pem \
  -days 365

# Start server with TLS
llm-memory-graph server start \
  --tls-cert server-cert.pem \
  --tls-key server-key.pem
```

### Client-side TLS

```typescript
import { MemoryGraphClient } from '@llm-dev-ops/llm-memory-graph-client';
import * as fs from 'fs';

const client = new MemoryGraphClient({
  address: 'server.example.com:50051',
  useTls: true,
  tlsOptions: {
    rootCerts: fs.readFileSync('ca-cert.pem'),
    certChain: fs.readFileSync('client-cert.pem'),
    privateKey: fs.readFileSync('client-key.pem')
  }
});
```

## Basic Authentication

```typescript
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  credentials: {
    username: 'admin',
    password: 'secret'
  }
});
```

## Token-based Authentication

```typescript
// Custom metadata with token
const client = new MemoryGraphClient({
  address: 'localhost:50051',
  metadata: {
    authorization: 'Bearer YOUR_TOKEN_HERE'
  }
});
```

## mTLS (Mutual TLS)

```typescript
const client = new MemoryGraphClient({
  address: 'server.example.com:50051',
  useTls: true,
  tlsOptions: {
    rootCerts: fs.readFileSync('ca-cert.pem'),
    certChain: fs.readFileSync('client-cert.pem'),
    privateKey: fs.readFileSync('client-key.pem')
  }
});
```

## Best Practices

1. Always use TLS in production
2. Rotate credentials regularly
3. Use strong passwords
4. Implement rate limiting
5. Monitor authentication failures
6. Use certificate pinning
7. Implement proper RBAC

## See Also

- [Deployment Guide](../../docs/DEPLOYMENT_GUIDE.md)
- [Security Best Practices](../API.md#error-handling)
