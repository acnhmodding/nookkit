/// Why a file could not be sized. Callers keep the game's own sizing for it (no table entry)
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Sarc(#[from] nookkit_sarc::Error),

    #[error(transparent)]
    Yaz0(#[from] Yaz0Error),

    #[error(transparent)]
    Zstd(#[from] ZstdError),

    #[error(transparent)]
    Bfres(#[from] BfresError),

    #[error(transparent)]
    Geometry(#[from] GeometryError),

    #[error(transparent)]
    Heap(#[from] HeapError),

    #[error(transparent)]
    Physics(#[from] PhysicsError),

    #[error(transparent)]
    Shader(#[from] ShaderError),

    #[error(transparent)]
    Table(#[from] TableError),

    #[error("out of memory reading {0}")]
    FileOutOfMemory(String),

    /// What could not be allocated, such as "Yaz0 data"
    #[error("out of memory for {0}")]
    OutOfMemory(&'static str),

    #[error("arithmetic operation resulted in an overflow")]
    Overflow,

    #[error("resource data exceeds its bounds, {count} bytes at 0x{at:X}")]
    OutOfBounds { at: i64, count: i64 },

    #[error("resource pointer 0x{value:X} at 0x{at:X} exceeds its bounds")]
    PointerOutOfBounds { at: i64, value: u64 },

    /// The magic that was expected, read as text
    #[error("expected {0}")]
    Magic(String),

    /// The resource's path, its size doesn't fit the table's 32 bits
    #[error("{0} is too big")]
    TooBig(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Yaz0Error {
    #[error("expected Yaz0 data")]
    NotYaz0,

    #[error("Yaz0 data is truncated")]
    Truncated,

    #[error("Yaz0 back-reference before the data start")]
    BackReference,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ZstdError {
    #[error("expected a Zstandard frame")]
    NotZstd,

    #[error("reserved Zstandard frame bit is set")]
    ReservedFrameBit,

    #[error("reserved Zstandard block type")]
    ReservedBlockType,

    #[error("Zstandard dictionaries are not supported")]
    Dictionary,

    #[error("truncated Zstandard frame")]
    Truncated,

    #[error("Zstandard window is too large")]
    WindowTooLarge,

    #[error("Zstandard data is too large")]
    TooLarge,

    #[error("Zstandard frames do not match their declared sizes")]
    SizeMismatch,

    /// The decoder's own error
    #[error("Zstandard data is invalid: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BfresError {
    #[error("external file arrays do not match their count")]
    ExternalFileCount,

    #[error("external file overlaps the relocation table")]
    ExternalFileOverlap,

    #[error("external files must be BNTX v4.1 textures")]
    ExternalFileNotTexture,

    #[error("BNTX has no textures")]
    NoTextures,

    #[error("animation arrays do not match their count")]
    AnimationCount,

    #[error("animation has no name")]
    UnnamedAnimation,

    #[error("unterminated BFRES string")]
    UnterminatedString,

    #[error("BFRES string contains an embedded terminator")]
    EmbeddedTerminator,

    #[error("BFRES string is not valid UTF-8")]
    InvalidUtf8,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    #[error("expected a canonical BFRES v9 model archive, optionally with output.phive")]
    NotModelArchive,

    #[error("no geometry models to analyze")]
    NoModels,

    /// The material's name
    #[error("no compiled static candidate for material {0}")]
    NoStaticCandidate(String),

    #[error("additional shader-option command bindings are unresolved")]
    UnresolvedCommandBindings,

    #[error("assign-variation shader profiles are unresolved")]
    UnresolvedAssignVariation,

    #[error("shape material or vertex reference is invalid")]
    InvalidShapeReference,

    #[error("mesh has no submeshes")]
    NoSubmeshes,

    #[error("missing render-pass shader option")]
    MissingRenderPass,

    #[error("no render-pass fallback")]
    NoRenderPassFallback,

    #[error("no compiled dynamic shader program")]
    NoDynamicProgram,

    #[error("material references a different shader archive")]
    ForeignShaderArchive,

    #[error("missing shading model {0}")]
    MissingShadingModel(String),

    #[error("duplicate material render info")]
    DuplicateRenderInfo,

    #[error("unknown material render mode")]
    UnknownRenderMode,

    #[error("unknown material culling mode")]
    UnknownCullingMode,

    #[error("duplicate vertex attribute")]
    DuplicateVertexAttribute,

    #[error("vertex stream or location is out of range")]
    VertexStreamOutOfRange,

    #[error("too many resource vertex streams")]
    TooManyVertexStreams,

    #[error("missing allocation metadata array")]
    MissingAllocationMetadata,

    #[error("allocation dictionary count mismatch")]
    AllocationCountMismatch,

    #[error("duplicate shader assignment")]
    DuplicateShaderAssignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HeapError {
    #[error("the initial free block must be 8-aligned")]
    UnalignedFreeBlock,

    #[error("heap requests cannot be negative")]
    NegativeRequest,

    #[error("only the native-verified positive alignments from 8 through 8192 are supported")]
    UnsupportedAlignment,

    #[error("GPU requests must be nonempty")]
    EmptyGpuRequest,

    #[error("GPU alignment must be a positive power of two")]
    GpuAlignmentNotPowerOfTwo,

    #[error("GPU alignments above the native virtual base are unresolved")]
    GpuAlignmentTooLarge,

    #[error("new GPU pool cannot satisfy its initial request")]
    GpuPoolTooSmall,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PhysicsError {
    /// The member's name, empty when it has none
    #[error("unsupported physics member {0}")]
    UnsupportedMember(String),

    #[error("helper bone parameters are missing")]
    MissingHelperBoneParameters,

    #[error("helper bone curves are missing")]
    MissingHelperBoneCurves,

    #[error("helper bone curve count exceeds its parameters")]
    HelperBoneCurveCount,

    #[error("fish bone parameters are missing")]
    MissingFishBoneParameters,

    #[error("cloth must be a little-endian Phive file")]
    NotLittleEndianCloth,

    #[error("unverified cloth operator {0}")]
    UnverifiedClothOperator(String),

    #[error("simulate operator has no simulation data")]
    NoSimulationData,

    #[error("expected an AAMP v2 parameter file")]
    NotAamp,

    #[error("physics parameter {0} is not an int")]
    NotInt(String),

    #[error("expected a Havok TAG0 tagfile")]
    NotTagfile,

    #[error("Havok tagfile is missing its data, types or items")]
    MissingTagfileSections,

    #[error("Havok type count is negative")]
    NegativeTypeCount,

    #[error("Havok type name is out of range")]
    TypeNameOutOfRange,

    #[error("Havok item is out of range")]
    ItemOutOfRange,

    #[error("Havok item index is out of range")]
    ItemIndexOutOfRange,

    #[error("expected Havok object {0}")]
    UnexpectedObject(String),

    #[error("multiple {0} objects")]
    MultipleObjects(String),

    #[error("missing Havok object {0}")]
    MissingObject(String),

    #[error("invalid Havok section size 0x{size:X} at 0x{at:X}")]
    InvalidSectionSize { at: i64, size: i64 },

    #[error("truncated Havok varint at 0x{at:X}")]
    TruncatedVarint { at: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShaderError {
    #[error("unterminated shader string")]
    UnterminatedString,

    #[error("shader dictionary is too large")]
    DictionaryTooLarge,

    /// The section's magic, read as text
    #[error("expected {0} shader section")]
    MissingSection(String),

    #[error("only the ACNH Switch BFSHA v8 layout is supported")]
    UnsupportedLayout,

    #[error("shader archive size mismatch")]
    SizeMismatch,

    #[error("invalid material uniform block index")]
    InvalidUniformBlock,

    #[error("duplicate compiled shader key")]
    DuplicateProgramKey,

    #[error("shader option index is out of range")]
    OptionOutOfRange,

    #[error("shader option references the wrong key region")]
    OptionWrongRegion,

    #[error("unknown shader choice {option}={value}")]
    UnknownChoice { option: String, value: String },

    #[error("shader choice exceeds its mask")]
    ChoiceExceedsMask,

    #[error("shader program index is out of range")]
    ProgramOutOfRange,

    #[error("missing compiled resource locations")]
    MissingResourceLocations,

    #[error("duplicate shader resource name")]
    DuplicateResourceName,

    #[error("invalid BNSH file size")]
    InvalidBnshSize,

    #[error("invalid shader option bit packing")]
    InvalidOptionPacking,

    #[error("expected one BFSHA file in the shader archive")]
    NotOneArchive,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    #[error("not a valid RSTC table")]
    Invalid,

    /// The entry's name hash
    #[error("duplicate table entry {0:08X}")]
    DuplicateEntry(u32),

    #[error("collided table name is not text")]
    CollidedNameNotText,

    #[error("invalid or duplicate table entry {0}")]
    InvalidCollidedEntry(String),

    #[error("{0} cannot be stored as a collided table name")]
    UnstorableName(String),
}

pub(crate) fn fail<T>(error: impl Into<Error>) -> Result<T> {
    Err(error.into())
}
