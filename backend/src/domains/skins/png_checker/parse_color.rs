use crate::png_checker::{ColorFormatInfo, ColorType, PLTERequirement, tRNSRequirement};

pub fn analyze_color_format(color_type: ColorType) -> ColorFormatInfo {
    match color_type {
        ColorType::GrayScale => ColorFormatInfo {
            has_alpha: false,
            plte_requirement: PLTERequirement::Disallowed,
            trns_requirement: tRNSRequirement::Allowed,
        },
        ColorType::TrueColor => ColorFormatInfo {
            has_alpha: false,
            plte_requirement: PLTERequirement::Optional,
            trns_requirement: tRNSRequirement::Allowed,
        },
        ColorType::IndexedColor => ColorFormatInfo {
            has_alpha: false,
            plte_requirement: PLTERequirement::Must,
            trns_requirement: tRNSRequirement::Allowed,
        },
        ColorType::GrayScaleAlpha => ColorFormatInfo {
            has_alpha: true,
            plte_requirement: PLTERequirement::Disallowed,
            trns_requirement: tRNSRequirement::Disallowed,
        },
        ColorType::TrueColorAlpha => ColorFormatInfo {
            has_alpha: true,
            plte_requirement: PLTERequirement::Optional,
            trns_requirement: tRNSRequirement::Disallowed,
        },
    }
}
