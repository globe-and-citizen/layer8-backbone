# layer8-backbone
This repo is for where the layer8 backbone will be mocked: frontend, FP, RP, backend, &amp; TIO server

## Setup

- One command to run all four services: (This requires concurrently to be installed, use the command `npm i -g concurrently` as root)

```bash
make run
```

- To run each service separately, use the following commands:

1. `make run-fp`
2. `make run-rp`
3. `make run-backend`
4. `make run-frontend`