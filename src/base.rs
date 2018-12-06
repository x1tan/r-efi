//! UEFI Base Environment
//!
//! This module defines the base environment for UEFI development. It provides types and macros as
//! declared in the UEFI specification, as well as de-facto standard additions provided by the
//! reference implementation by Intel.
//!
//! # Target Configuration
//!
//! Wherever possible, native rust types are used to represent their UEFI counter-parts. However,
//! this means the ABI depends on the implementation of said rust types. Hence, native rust types
//! are only used where rust supports a stable ABI of said types, and their ABI matches the ABI
//! defined by the UEFI specification.
//!
//! Nevertheless, even if the ABI of a specific type is marked stable, this does not imply that it
//! is the same across architectures. For instance, rust's `u64` type has the same binary
//! representation as the `UINT64` type in UEFI. But this does not imply that it has the same
//! binary representation on `x86_64` and on `ppc64be`. As a result of this, the compilation of
//! this module is tied to the target-configuration you passed to the rust compiler. Wherever
//! possible and reasonable, any architecture differences are abstracted, though. This means that
//! in most cases you can use this module even though your target-configuration might not match
//! the native UEFI target-configuration.
//!
//! The recommend way to compile your code, is to use the native target-configuration for UEFI.
//! These configurations are not necessarily included in the upstream rust compiler. Hence, you
//! might have to craft one yourself. This project contains target-configurations for all targets
//! defined by the UEFI specification and supported by rust.
//!
//! However, there are situations where you want to access UEFI data from a non-native host. For
//! instance, a UEFI boot loader might store data in boot variables, formatted according to types
//! declared in the UEFI specification. An OS booted thereafter might want to access these
//! variables, but it might be compiled with a different target-configuration than the UEFI
//! environment that it was booted from. A similar situation occurs when you call UEFI runtime
//! functions from your OS. In all those cases, you should very likely be able to use this module
//! to interact with UEFI as well. This is, because most bits of the target-configuration of UEFI
//! and your OS very likely match. In fact, to figure out whether this is safe, you need to make
//! sure that the rust ABI would match in both target-configurations. If it is, all other details
//! are handled within this module just fine.
//!
//! In case of doubt, contact us!
//!
//! # Core Primitives
//!
//! Several of the UEFI primitives are represented by native Rust. These have no type aliases or
//! other definitions here, but you are recommended to use native rust directly. These include:
//!
//!  * `NULL`, `void *`: Void pointers have a native rust implementation in
//!                      [`c_void`](core::ffi::c_void). `NULL` is represented through
//!                      [`null`](core::ptr::null) and [`is_null()`](core::ptr) for
//!                      all pointers types.
//!  * `uint8_t`..`uint64_t`,
//!    `int8_t`..`int64_t`: Fixed-size integers are represented by their native rust equivalents
//!                         (`u8`..`u64`, `i8`..`i64`).
//!
//!  * `UINTN`, `INTN`: Native-sized (or instruction-width sized) integers are represented by
//!                     their native rust equivalents (`usize`, `isize`).
//!
//! # UEFI Details
//!
//! The UEFI Specification describes its target environments in detail. Each supported
//! architecture has a separate section with details on calling conventions, CPU setup, and more.
//! You are highly recommended to conduct the UEFI Specification for details on the programming
//! environment. Following a summary of key parts relevant to rust developers:
//!
//!  * Similar to rust, integers are either fixed-size, or native size. This maps nicely to the
//!    native rust types. The common `long`, `int`, `short` types known from ISO-C are not used.
//!    Whenever you refer to memory (either pointing to it, or remember the size of a memory
//!    block), the native size integers should be your tool of choice.
//!
//!  * Even though the CPU might run in any endianness, all stored data is little-endian. That
//!    means, if you encounter integers split into byte-arrays (e.g.,
//!    `CEfiDevicePathProtocol.length`), you must assume it is little-endian encoded. But if you
//!    encounter native integers, you must assume they are encoded in native endianness.
//!    For now the UEFI specification only defines little-endian architectures, hence this did not
//!    pop up as actual issue. Future extensions might change this, though.
//!
//!  * The C-language-calling-convention is used. That is, all external calls to UEFI functions
//!    use the C calling convention. All such ABI functions must be marked as `extern "C"`.
//!    The UEFI Specification defines some additional common rules for all its APIs, though. You
//!    will most likely not see any of these mentioned in the individual API documentions. So here
//!    is a short reminder:
//!
//!     * Pointers must reference physical-memory locations (no I/O mappings, no
//!       virtual addresses, etc.). Once ExitBootServices() was called, and the
//!       virtual address mapping was set, you must provide virtual-memory
//!       locations instead.
//!     * Pointers must be correctly aligned.
//!     * NULL is disallowed, unless explicitly mentioned otherwise.
//!     * Data referenced by pointers is undefined on error-return from a
//!       function.
//!     * You must not pass data larger than native-size (sizeof(CEfiUSize)) on
//!       the stack. You must pass them by reference.
//!
//!  * Stack size is at least 128KiB and 16-byte aligned. All stack space might be marked
//!    non-executable! Once ExitBootServices() was called, you must guarantee at least 4KiB of
//!    stack space, 16-byte aligned for all runtime services you call.
//!    Details might differ depending on architectures. But the numbers here should serve as
//!    ball-park figures.

// Target Architecture
//
// The UEFI Specification explicitly lists all supported target architectures. While external
// implementors are free to port UEFI to other targets, we need information on the target
// architecture to successfully compile for it. This includes calling-conventions, register
// layouts, endianness, and more. Most of these details are hidden in the rust-target-declaration.
// However, some details are still left to the actual rust code.
//
// This initial check just makes sure the compilation is halted with a suitable error message if
// the target architecture is not supported.
//
// We try to minimize conditional compilations as much as possible. A simple search for
// `target_arch` should reveal all uses throughout the code-base. If you add your target to this
// error-check, you must adjust all other uses as well.
//
// Similarly, UEFI only defines configurations for little-endian architectures so far. Several
// bits of the specification are thus unclear how they would be applied on big-endian systems. We
// therefore mark it as unsupported. If you override this, you are on your own.
#[cfg(not(any(target_arch = "arm",
              target_arch = "aarch64",
              target_arch = "x86",
              target_arch = "x86_64")))]
compile_error!("The target architecture is not supported.");
#[cfg(not(any(target_endian = "little")))]
compile_error!("The target endianness is not supported.");

// eficall_arch!()
//
// This macro is the architecture-dependent implementation of eficall!(). See the documentation of
// the eficall!() macro for a description. We need to split the exported wrapper from the internal
// backend to make rustdoc attach to the right symbol.

#[cfg(target_arch = "arm")]
macro_rules! eficall_arch {
    (fn $in:tt $(-> $out:ty)?) => { extern "aapcs" fn $in $( -> $out )? };
}

// XXX: Rust does not define aapcs64, yet. Once it does, we should switch to it, rather than
//      referring to the system default.
#[cfg(target_arch = "aarch64")]
macro_rules! eficall_arch {
    (fn $in:tt $(-> $out:ty)?) => { extern "C" fn $in $( -> $out )? };
}

#[cfg(target_arch = "x86")]
macro_rules! eficall_arch {
    (fn $in:tt $(-> $out:ty)?) => { extern "cdecl" fn $in $( -> $out )? };
}

#[cfg(target_arch = "x86_64")]
macro_rules! eficall_arch {
    (fn $in:tt $(-> $out:ty)?) => { extern "win64" fn $in $( -> $out )? };
}

#[cfg(not(any(target_arch = "arm",
              target_arch = "aarch64",
              target_arch = "x86",
              target_arch = "x86_64")))]
macro_rules! eficall_arch {
    (fn $in:tt $(-> $out:ty)?) => { extern "C" fn $in $( -> $out )? };
}

/// Annotate function with UEFI calling convention
///
/// This macro takes a function-declaration as argument and produces the same function-declaration
/// but annotated with the correct calling convention. Since the default `extern "C"` annotation
/// depends on your compiler defaults, we cannot use it. Instead, this macro selects the default
/// for your target platform.
///
/// # Calling Conventions
///
/// The UEFI specification defines the calling convention for each platform individually. It
/// usually refers to other standards for details, but adds some restrictions on top. As of this
/// writing, it mentions:
///
///  * aarch32 / arm: The `aapcs` calling-convention is used. It is native to aarch32 and described
///                   in a document called
///                   "Procedure Call Standard for the ARM Architecture". It is openly distributed
///                   by ARM and widely known under the keyword `aapcs`.
///  * aarch64: The `aapcs64` calling-convention is used. It is native to aarch64 and described in
///             a document called
///             "Procedure Call Standard for the ARM 64-bit Architecture (AArch64)". It is openly
///             distributed by ARM and widely known under the keyword `aapcs64`.
///  * ia-64: The "P64 C Calling Convention" as described in the
///           "Itanium Software Conventions and Runtime Architecture Guide". It is also
///           standardized in the "Intel Itanium SAL Specification".
///  * RISC-V: The "Standard RISC-V C Calling Convention" is used. The UEFI specification
///            describes it in detail, but also refers to the official RISC-V resources for
///            detailed information.
///  * x86 / ia-32: The `cdecl` C calling convention is used. Originated in the C Language and
///                 originally tightly coupled to C specifics. Unclear whether a formal
///                 specification exists (does anyone know?). Most compilers support it under the
///                 `cdecl` keyword, and in nearly all situations it is the default on x86.
///  * x86_64 / amd64 / x64: The `win64` calling-convention is used. It is similar to the `sysv64`
///                          convention that is used on most non-windows x86_64 systems, but not
///                          exactly the same. Microsoft provides open documentation on it. See
///                          MSDN "x64 Software Conventions -> Calling Conventions".
///                          The UEFI Specification does not directly refer to `win64`, but
///                          contains a full specification of the calling convention itself.
///
/// Note that in most cases the UEFI Specification adds several more restrictions on top of the
/// common calling-conventions. These restrictions usually do not affect how the compiler will lay
/// out the function calls. Instead, it usually only restricts the set of APIs that are allowed in
/// UEFI. Therefore, most compilers already support the calling conventions used on UEFI.
///
/// # Variadics
///
/// For some reason, the rust compiler allows variadics only in combination with the `"C"` calling
/// convention, even if the selected calling-convention matches what `"C"` would select on the
/// target platform. Hence, we do not support variadics so far. Luckily, all of the UEFI functions
/// that use variadics are wrappers around more low-level accessors, so they are not necessarily
/// required.
#[macro_export]
macro_rules! eficall {
    ($($arg:tt)*) => { eficall_arch!($($arg)*) };
}

/// Boolean Type
///
/// This boolean type works very similar to the rust primitive type of [`bool`]. However, the rust
/// primitive type has no stable ABI, hence we provide this type to represent booleans on the FFI
/// interface.
///
/// UEFI defines booleans to be 1-byte integers, which can only have the values of `0` or `1`.
/// This enum provides the equivalent definitions as [`Boolean::False`] and [`Boolean::True`].
#[repr(u8)]
pub enum Boolean {
    False = 0u8,
    True = 1u8,
}

/// Single-byte Character Type
///
/// The `Char8` type represents single-byte characters. UEFI defines them to be ASCII compatible,
/// using the ISO-Latin-1 character set.
pub type Char8 = u8;

/// Dual-byte Character Type
///
/// The `Char16` type represents dual-byte characters. UEFI defines them to be UCS-2 encoded.
pub type Char16 = u16;

/// Globally Unique Identifiers
///
/// The `Guid` type represents globally unique identifiers as defined by RFC-4122 (i.e., only the
/// `10x` variant is used). The type must be 64-bit aligned.
///
/// Note that only the binary representation of Guids is stable. You are highly recommended to
/// interpret Guids as 128bit integers.
///
/// UEFI uses the Microsoft-style Guid format. Hence, a lot of documentation and code refers to
/// these Guids. If you thusly cannot treat Guids as 128-bit integers, this Guid type allows you
/// to access the individual fields of the Microsoft-style Guid. A reminder of the Guid encoding:
///
/// ```text
///    0                   1                   2                   3
///    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                          time_low                             |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |       time_mid                |         time_hi_and_version   |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |clk_seq_hi_res |  clk_seq_low  |         node (0-1)            |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                         node (2-5)                            |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// The individual fields are encoded as big-endian (network-byte-order). The Guid structure
/// allows you direct access to these fields. Make sure to convert endianness when accessing the
/// data. Data stored in Guid objects must be considered big-endian.
#[repr(C, align(8))]
pub struct Guid {
    pub time_low: u32,
    pub time_mid: u16,
    pub time_hi_and_version: u16,
    pub clk_seq_hi_res: u8,
    pub clk_seq_low: u8,
    pub node: [u8; 6],
}

/// Status Codes
///
/// UEFI uses the `Status` type to represent all kinds of status codes. This includes return codes
/// from functions, but also complex state of different devices and drivers. It is a simple
/// `usize`. Depending on the context, different state is stored in it.
pub type Status = usize;

/// Object Handles
///
/// Handles represent access to an opaque object. Handles are untyped by default, but get a
/// meaning when you combine them with an interface. Internally, they are simple void pointers. It
/// is the UEFI driver model that applies meaning to them.
pub type Handle = *mut core::ffi::c_void;

/// Event Objects
///
/// Event objects represent hooks into the main-loop of a UEFI environment. They allow to register
/// callbacks, to be invoked when a specific event happens. In most cases you use events to
/// register timer-based callbacks, as well as chaining events together. Internally, they are
/// simple void pointers. It is the UEFI task management that applies meaning to them.
pub type Event = *mut core::ffi::c_void;

/// Logical Block Addresses
///
/// The LBA type is used to denote logical block addresses of block devices. It is a simple 64-bit
/// integer, that is used to denote addresses when working with block devices.
pub type Lba = u64;

/// Thread Priority Levels
///
/// The process model of UEFI systems is highly simplified. Priority levels are used to order
/// execution of pending tasks. The TPL type denotes a priority level of a specific task. The
/// higher the number, the higher the priority. It is a simple integer type, but its range is
/// usually highly restricted. The UEFI task management provides constants and accessors for TPLs.
pub type Tpl = usize;

/// Physical Memory Address
///
/// A simple 64bit integer containing a physical memory address.
pub type PhysicalAddress = u64;

/// Virtual Memory Address
///
/// A simple 64bit integer containing a virtual memory address.
pub type VirtualAddress = u64;

/// Application Entry Point
///
/// This type defines the entry-point of UEFI applications. It is ABI and cannot be changed.
/// Whenever you load UEFI images, the entry-point is called with this signature.
///
/// In most cases the UEFI image (or application) is unloaded when control returns from the entry
/// point. In case of UEFI drivers, they can request to stay loaded until an explicit unload.
///
/// The system table is provided as mutable pointer. This is, because there is no guarantee that
/// timer interrupts do not modify the table. Furthermore, exiting boot services causes several
/// modifications on that table. And lastly, the system table lives longer than the function
/// invocation, if invoked as an UEFI driver.
/// In most cases it is perfectly fine to cast the pointer to a real rust reference. However, this
/// should be an explicit decision by the caller.
pub type ImageEntryPoint = fn(Handle, *mut crate::system::SystemTable) -> Status;

impl Guid {
    /// Initialize a Guid from its individual fields in native endianness
    ///
    /// This is the most basic initializer of a Guid object. It takes the individual fields in
    /// native endian and creates the big-endian Guid object with it.
    pub const fn from_native(
        time_low: u32,
        time_mid: u16,
        time_hi_and_version: u16,
        clk_seq_hi_res: u8,
        clk_seq_low: u8,
        node: &[u8; 6],
    ) -> Guid {
        Guid {
            time_low: time_low.to_be(),
            time_mid: time_mid.to_be(),
            time_hi_and_version: time_hi_and_version.to_be(),
            clk_seq_hi_res: clk_seq_hi_res.to_be(),
            clk_seq_low: clk_seq_low.to_be(),
            node: *node,
        }
    }

    /// Initialize a Guid from its individual fields as given in the specification
    ///
    /// This function initializes a Guid object given the individual fields as specified in the
    /// UEFI specification. That is, if you simply copy the literals from the specification into
    /// your code, this function will correctly initialize the Guid object.
    ///
    /// The UEFI specification provides definitions of Guids as a set of integer literals. They
    /// are meant to be directly assigned to the corresponding Guid fields. However, the
    /// specification assumes little-endian systems, therefore the literals are provided in
    /// big-endian format, so the conversion can be skipped. This will not work on big-endian
    /// systems, though. Hence, this function applies the required conversions.
    pub const fn from_spec(
        time_low: u32,
        time_mid: u16,
        time_hi_and_version: u16,
        clk_seq_hi_res: u8,
        clk_seq_low: u8,
        node: &[u8; 6],
    ) -> Guid {
        // The literals are given in inverted byte-order in the specification. Revert this to
        // native order and then simply use the basic constructor.
        Guid::from_native(
            time_low.swap_bytes(),
            time_mid.swap_bytes(),
            time_hi_and_version.swap_bytes(),
            clk_seq_hi_res.swap_bytes(),
            clk_seq_low.swap_bytes(),
            node,
        )
    }

    /// Access a Guid as raw byte array
    ///
    /// This provides access to a Guid through a byte array. It is a simple re-interpretation of
    /// the Guid value as a 128-bit byte array. No conversion is performed. This is a simple cast.
    pub fn as_bytes(&self) -> &[u8; 16] {
        unsafe {
            core::mem::transmute::<&Guid, &[u8; 16]>(self)
        }
    }
}