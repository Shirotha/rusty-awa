use crate::u5;
use bitbuffer::{BitRead, BitWrite};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BitRead, BitWrite)]
#[discriminant_bits = 5]
pub enum AwaTism {
    #[discriminant = 0x00]
    NoOp,
    #[discriminant = 0x01]
    Print,
    #[discriminant = 0x02]
    PrintNum,
    #[discriminant = 0x03]
    Read,
    #[discriminant = 0x04]
    ReadNum,
    #[discriminant = 0x1F]
    Terminate,
    #[discriminant = 0x05]
    Blow(i8),
    #[discriminant = 0x06]
    Submerge(u5),
    #[discriminant = 0x07]
    Pop,
    #[discriminant = 0x08]
    Duplicate,
    #[discriminant = 0x09]
    Surround(u5),
    #[discriminant = 0x0A]
    Merge,
    #[discriminant = 0x0B]
    Add,
    #[discriminant = 0x0C]
    Subtract,
    #[discriminant = 0x0D]
    Multiply,
    #[discriminant = 0x0E]
    Divide,
    #[discriminant = 0x0F]
    Count,
    #[discriminant = 0x10]
    Label(u5),
    #[discriminant = 0x11]
    Jump(u5),
    #[discriminant = 0x12]
    EqualTo,
    #[discriminant = 0x13]
    LessThan,
    #[discriminant = 0x14]
    GreaterThan,
    #[discriminant = 0x16]
    DoublePop,
}
impl Display for AwaTism {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOp => f.write_str("nop"),
            Self::Print => f.write_str("prn"),
            Self::PrintNum => f.write_str("pr1"),
            Self::Read => f.write_str("red"),
            Self::ReadNum => f.write_str("r3d"),
            Self::Terminate => f.write_str("trm"),
            Self::Blow(value) => f.write_fmt(format_args!("blo {}", value)),
            Self::Submerge(distance) => f.write_fmt(format_args!("sbm {}", distance)),
            Self::Pop => f.write_str("pop"),
            Self::Duplicate => f.write_str("dpl"),
            Self::Surround(count) => f.write_fmt(format_args!("srn {}", count)),
            Self::Merge => f.write_str("mrg"),
            Self::Add => f.write_str("4dd"),
            Self::Subtract => f.write_str("sub"),
            Self::Multiply => f.write_str("mul"),
            Self::Divide => f.write_str("div"),
            Self::Count => f.write_str("cnt"),
            Self::Label(label) => f.write_fmt(format_args!("lbl {}", label)),
            Self::Jump(label) => f.write_fmt(format_args!("jmp {}", label)),
            Self::EqualTo => f.write_str("eql"),
            Self::LessThan => f.write_str("lss"),
            Self::GreaterThan => f.write_str("gr8"),
            Self::DoublePop => f.write_str("p0p"),
        }
    }
}
