# Language Reference

This document provides a comprehensive reference for the Pyra programming language.

## Types

### Basic Types
- `uint256`: Unsigned 256-bit integer
- `int256`: Signed 256-bit integer  
- `bool`: Boolean (true/false)
- `address`: Ethereum address
- `bytes`: Byte array

### Complex Types
- `struct`: Custom data structures
- `Vec<T>`: Dynamic arrays (compile-time optimized)
- `Map<K,V>`: Key-value mappings (compile-time optimized)

## Syntax

### Variable Declaration
```pyra
let variable_name: type = value
mut variable_name: type = value  # mutable
const CONSTANT_NAME: type = value  # compile-time constant
```

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

while condition:
    # loop body
```

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

def generic_function<T: Constraint>(param: T) -> T:
    return param
```