use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Pod, Zeroable, PartialOrd, Ord)]
pub struct IdEntri(pub u32);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Pod, Zeroable, PartialOrd, Ord)]
pub struct IdNama(pub u32);

/// Penanda bahwa entri tidak memiliki induk (adalah Root).
/// Disimpan sebagai u32 mentah di RekamanEntri.induk (bukan enum) agar tetap Pod.
pub const INDUK_AKAR: u32 = u32::MAX;

