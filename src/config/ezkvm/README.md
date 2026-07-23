# Design

## Directories:
- file: This directory contains types specific for reading / writing Ezkvm config files as well as serializing/deserializing them. It should not depend on types related to runtime.
- schema: This directory contains Ezkvm config schema specific types, should not depend on types related to file io or runtime.
- runtime: This directory contains types specific for translating from Ezkvm schema to Runtime and vice versa.

## Design

```plantuml
@startuml
namespace src.config.ezkvm.file {
    struct ConfigFileStore {

    }
    struct Builder {

    }
    struct Parser {

    }

    ConfigFileStore --> Builder
    ConfigFileStore --> Parser
}

namespace src.config.ezkvm.runtime {
    struct Builder {

    }
    struct Parser {

    }
}

namespace src.config.ezkvm.schema {
    struct ConfigSchema
}

namespace src.config.ezkvm {
    runtime.Builder ..> schema.ConfigSchema
    runtime.Parser ..> schema.ConfigSchema
    file.ConfigFileStore ..> schema.ConfigSchema
}

namespace src.runtime {
    struct Runtime
}

src.runtime.Runtime <.. src.config.ezkvm.runtime.Builder
src.runtime.Runtime <.. src.config.ezkvm.runtime.Parser

@enduml
```