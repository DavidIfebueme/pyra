# Language Reference

This document describes the Pyra language as implemented by the current compiler.

Sections marked “Planned” are not implemented yet.

## Types

### Basic Types
- `uint256`: Unsigned 256-bit integer
- `int256`: Signed 256-bit integer  
- `bool`: Boolean (true/false)
- `address`: Ethereum address
- `bytes`: Byte array
- `string`: String

### Complex Types
- `struct`: Custom data structures
- `Vec<T>`: Planned
- `Map<K,V>`: Planned

## Syntax

### Variable Declaration
```pyra
let variable_name: type = value
let mut variable_name: type = value
const CONSTANT_NAME: type = value
```

Note: at top-level, `let NAME: type = value` is also accepted as a constant declaration for now (used by existing examples).

### Function Definition
```pyra
def function_name(param1: type1, param2: type2) -> return_type:
    # function body
    return value
```

### Control Flow
```pyra
if condition:
    # code block
elif other_condition:
    # code block
else:
    # code block
```

Planned:

- `while`

### Struct Definition
```pyra
struct StructName {
    field1: type1,
    field2: type2
}
```

### Generic Types
```pyra
struct Container<T> {
    value: T
}
```

Planned:

- Generic functions
- Generic type constraints