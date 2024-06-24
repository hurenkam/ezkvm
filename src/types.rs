pub enum EzkvmError {
    OpenError { file: String },
    ReadError { file: String },
    WriteError { file: String },
    ExecError { file: String },
    DeleteError { file: String },
    ParseError { file: String },
    ResourceNotAvailable { pool: String }
}
