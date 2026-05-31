# Python SCON

Python implementation of SCON core parsing, resolution, canonical value
formatting, and dataclass-based typed encoding/decoding.

```python
from dataclasses import dataclass
import scon

@dataclass
class Config:
    name: str
    port: int

config = scon.from_scon('name = "demo"\nport = 8080', Config)
```

Useful commands:

```sh
python -m pytest
```
