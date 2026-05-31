# TypeScript SCON

TypeScript implementation of SCON core parsing, resolution, canonical value
formatting, and Zod-based typed decoding/encoding.

```ts
import { parseString } from "@realmlabs/scon-core";

const value = parseString('name = "demo"');
```

Useful commands:

```sh
pnpm install
pnpm test
pnpm typecheck
```
