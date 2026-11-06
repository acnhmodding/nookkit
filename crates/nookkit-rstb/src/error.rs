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

    #[error("Out of memory reading {0}.")]
    FileOutOfMemory(String),
    /// What could not be allocated, such as "Yaz0 data"
    #[error("Out of memory for {0}.")]
    OutOfMemory(&'static str),
    #[error("Arithmetic operation resulted in an overflow.")]
    Overflow,
    #[error("Resource data exceeds its bounds.")]
    OutOfBounds,
    #[error("Resource pointer exceeds its bounds.")]
    PointerOutOfBounds,
    /// The magic that was expected, read as text
    #[error("Expected {0}")]
    Magic(String),
    /// The resource's path, its size doesn't fit the table's 32 bits
    #[error("{0} is too big!")]
    TooBig(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Yaz0Error {
    #[error("Expected Yaz0 data.")]
    NotYaz0,
    #[error("Yaz0 data is truncated.")]
    Truncated,
    #[error("Yaz0 back-reference before the data start.")]
    BackReference,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ZstdError {
    #[error("Expected a Zstandard frame.")]
    NotZstd,
    #[error("Reserved Zstandard frame bit is set.")]
    ReservedFrameBit,
    #[error("Reserved Zstandard block type.")]
    ReservedBlockType,
    #[error("Zstandard dictionaries are not supported.")]
    Dictionary,
    #[error("Truncated Zstandard frame.")]
    Truncated,
    #[error("Zstandard window is too large.")]
    WindowTooLarge,
    #[error("Zstandard data is too large.")]
    TooLarge,
    #[error("Zstandard frames do not match their declared sizes.")]
    SizeMismatch,
    /// The decoder's own error
    #[error("Zstandard data is invalid: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BfresError {
    #[error("External file arrays do not match their count.")]
    ExternalFileCount,
    #[error("External file overlaps the relocation table.")]
    ExternalFileOverlap,
    #[error("External files must be BNTX v4.1 textures.")]
    ExternalFileNotTexture,
    #[error("BNTX has no textures.")]
    NoTextures,
    #[error("Animation arrays do not match their count.")]
    AnimationCount,
    #[error("Animation has no name.")]
    UnnamedAnimation,
    #[error("Unterminated BFRES string.")]
    UnterminatedString,
    #[error("BFRES string contains an embedded terminator.")]
    EmbeddedTerminator,
    #[error("BFRES string is not valid UTF-8.")]
    InvalidUtf8,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    #[error("Expected a canonical BFRES v9 model archive, optionally with output.phive.")]
    NotModelArchive,
    #[error("No geometry models to analyze.")]
    NoModels,
    /// The material's name
    #[error("No compiled static candidate for material {0}")]
    NoStaticCandidate(String),
    #[error("Additional shader-option command bindings are unresolved.")]
    UnresolvedCommandBindings,
    #[error("Assign-variation shader profiles are unresolved.")]
    UnresolvedAssignVariation,
    #[error("Shape material or vertex reference is invalid.")]
    InvalidShapeReference,
    #[error("Mesh has no submeshes.")]
    NoSubmeshes,
    #[error("Missing render-pass shader option.")]
    MissingRenderPass,
    #[error("No render-pass fallback.")]
    NoRenderPassFallback,
    #[error("No compiled dynamic shader program.")]
    NoDynamicProgram,
    #[error("Material references a different shader archive.")]
    ForeignShaderArchive,
    #[error("Missing shading model {0}")]
    MissingShadingModel(String),
    #[error("Duplicate material render info.")]
    DuplicateRenderInfo,
    #[error("Unknown material render mode.")]
    UnknownRenderMode,
    #[error("Unknown material culling mode.")]
    UnknownCullingMode,
    #[error("Duplicate vertex attribute.")]
    DuplicateVertexAttribute,
    #[error("Vertex stream or location is out of range.")]
    VertexStreamOutOfRange,
    #[error("Too many resource vertex streams.")]
    TooManyVertexStreams,
    #[error("Missing allocation metadata array.")]
    MissingAllocationMetadata,
    #[error("Allocation dictionary count mismatch.")]
    AllocationCountMismatch,
    #[error("Duplicate shader assignment.")]
    DuplicateShaderAssignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HeapError {
    #[error("The initial free block must be 8-aligned.")]
    UnalignedFreeBlock,
    #[error("Heap requests cannot be negative.")]
    NegativeRequest,
    #[error("Only the native-verified positive alignments from 8 through 8192 are supported.")]
    UnsupportedAlignment,
    #[error("GPU requests must be nonempty.")]
    EmptyGpuRequest,
    #[error("GPU alignment must be a positive power of two.")]
    GpuAlignmentNotPowerOfTwo,
    #[error("GPU alignments above the native virtual base are unresolved.")]
    GpuAlignmentTooLarge,
    #[error("New GPU pool cannot satisfy its initial request.")]
    GpuPoolTooSmall,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PhysicsError {
    /// The member's name, empty when it has none
    #[error("Unsupported physics member {0}")]
    UnsupportedMember(String),
    #[error("Helper bone parameters are missing.")]
    MissingHelperBoneParameters,
    #[error("Helper bone curves are missing.")]
    MissingHelperBoneCurves,
    #[error("Helper bone curve count exceeds its parameters.")]
    HelperBoneCurveCount,
    #[error("Fish bone parameters are missing.")]
    MissingFishBoneParameters,
    #[error("Cloth must be a little-endian Phive file.")]
    NotLittleEndianCloth,
    #[error("Unverified cloth operator {0}")]
    UnverifiedClothOperator(String),
    #[error("Simulate operator has no simulation data.")]
    NoSimulationData,
    #[error("Expected an AAMP v2 parameter file.")]
    NotAamp,
    #[error("Physics parameter {0} is not an int.")]
    NotInt(String),
    #[error("Expected a Havok TAG0 tagfile.")]
    NotTagfile,
    #[error("Havok tagfile is missing its data, types or items.")]
    MissingTagfileSections,
    #[error("Havok type count is negative.")]
    NegativeTypeCount,
    #[error("Havok type name is out of range.")]
    TypeNameOutOfRange,
    #[error("Havok item is out of range.")]
    ItemOutOfRange,
    #[error("Havok item index is out of range.")]
    ItemIndexOutOfRange,
    #[error("Expected Havok object {0}")]
    UnexpectedObject(String),
    #[error("Multiple {0} objects.")]
    MultipleObjects(String),
    #[error("Missing Havok object {0}")]
    MissingObject(String),
    #[error("Invalid Havok section size.")]
    InvalidSectionSize,
    #[error("Truncated Havok varint.")]
    TruncatedVarint,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShaderError {
    #[error("Unterminated shader string.")]
    UnterminatedString,
    #[error("Shader dictionary is too large.")]
    DictionaryTooLarge,
    /// The section's magic, read as text
    #[error("Expected {0} shader section.")]
    MissingSection(String),
    #[error("Only the ACNH Switch BFSHA v8 layout is supported.")]
    UnsupportedLayout,
    #[error("Shader archive size mismatch.")]
    SizeMismatch,
    #[error("Invalid material uniform block index.")]
    InvalidUniformBlock,
    #[error("Duplicate compiled shader key.")]
    DuplicateProgramKey,
    #[error("Shader option index is out of range.")]
    OptionOutOfRange,
    #[error("Shader option references the wrong key region.")]
    OptionWrongRegion,
    #[error("Unknown shader choice {option}={value}.")]
    UnknownChoice { option: String, value: String },
    #[error("Shader choice exceeds its mask.")]
    ChoiceExceedsMask,
    #[error("Shader program index is out of range.")]
    ProgramOutOfRange,
    #[error("Missing compiled resource locations.")]
    MissingResourceLocations,
    #[error("Duplicate shader resource name.")]
    DuplicateResourceName,
    #[error("Invalid BNSH file size.")]
    InvalidBnshSize,
    #[error("Invalid shader option bit packing.")]
    InvalidOptionPacking,
    #[error("Expected one BFSHA file in the shader archive.")]
    NotOneArchive,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    #[error("Not a valid RSTC table.")]
    Invalid,
    /// The entry's name hash
    #[error("Duplicate table entry {0:08X}.")]
    DuplicateEntry(u32),
    #[error("Collided table name is not text.")]
    CollidedNameNotText,
    #[error("Invalid or duplicate table entry {0}.")]
    InvalidCollidedEntry(String),
    #[error("{0} cannot be stored as a collided table name.")]
    UnstorableName(String),
}

pub(crate) fn fail<T>(error: impl Into<Error>) -> Result<T> {
    Err(error.into())
}
