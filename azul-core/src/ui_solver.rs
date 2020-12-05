use std::{fmt, collections::BTreeMap};
use azul_css::{
    LayoutRect, LayoutPoint, LayoutSize, PixelValue, StyleFontSize,
    StyleTextColor, ColorU as StyleColorU,
    StyleTextAlignmentHorz, StyleTextAlignmentVert,
};
use crate::{
    app_resources::{Words, ScaledWords, FontInstanceKey, WordPositions, LayoutedGlyphs},
    id_tree::{NodeId, NodeDataContainer},
    dom::{DomHash, ScrollTagId},
    callbacks::PipelineId,
};

pub const DEFAULT_FONT_SIZE_PX: isize = 16;
pub const DEFAULT_FONT_SIZE: StyleFontSize = StyleFontSize { inner: PixelValue::const_px(DEFAULT_FONT_SIZE_PX) };
pub const DEFAULT_FONT_ID: &str = "serif";
pub const DEFAULT_FONT_COLOR: StyleTextColor = StyleTextColor { inner: StyleColorU { r: 0, b: 0, g: 0, a: 255 } };
pub const DEFAULT_LINE_HEIGHT: f32 = 1.0;
pub const DEFAULT_WORD_SPACING: f32 = 1.0;
pub const DEFAULT_LETTER_SPACING: f32 = 0.0;
pub const DEFAULT_TAB_WIDTH: f32 = 4.0;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InlineTextLayout {
    pub lines: Vec<InlineTextLine>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct InlineTextLine {
    pub bounds: LayoutRect,
    /// At which word does this line start?
    pub word_start: usize,
    /// At which word does this line end
    pub word_end: usize,
}

impl InlineTextLine {
    pub const fn new(bounds: LayoutRect, word_start: usize, word_end: usize) -> Self {
        Self { bounds, word_start, word_end }
    }
}

impl InlineTextLayout {

    pub fn get_leading(&self) -> f32 {
        match self.lines.first() {
            None => 0.0,
            Some(s) => s.bounds.origin.x,
        }
    }

    pub fn get_trailing(&self) -> f32 {
        match self.lines.first() {
            None => 0.0,
            Some(s) => s.bounds.origin.x + s.bounds.size.width,
        }
    }

    pub const fn new(lines: Vec<InlineTextLine>) -> Self {
        Self { lines }
    }

    #[inline]
    #[must_use = "get_bounds calls union(self.lines) and is expensive to call"]
    pub fn get_bounds(&self) -> LayoutRect {
        LayoutRect::union(self.lines.iter().map(|c| c.bounds)).unwrap_or(LayoutRect::zero())
    }

    #[must_use = "function is expensive to call since it iterates + collects over self.lines"]
    pub fn get_children_horizontal_diff_to_right_edge(&self, parent: &LayoutRect) -> Vec<f32> {
        let parent_right_edge = parent.origin.x + parent.size.width;
        let parent_left_edge = parent.origin.x;
        self.lines.iter().map(|line| {
            let child_right_edge = line.bounds.origin.x + line.bounds.size.width;
            let child_left_edge = line.bounds.origin.x;
            (child_left_edge - parent_left_edge) + (parent_right_edge - child_right_edge)
        }).collect()
    }

    /// Align the lines horizontal to *their bounding box*
    pub fn align_children_horizontal(&mut self, horizontal_alignment: StyleTextAlignmentHorz) {
        let shift_multiplier = match calculate_horizontal_shift_multiplier(horizontal_alignment) {
            None =>  return,
            Some(s) => s,
        };
        let self_bounds = self.get_bounds();
        let horz_diff = self.get_children_horizontal_diff_to_right_edge(&self_bounds);

        for (line, shift) in self.lines.iter_mut().zip(horz_diff.into_iter()) {
            line.bounds.origin.x += shift * shift_multiplier;
        }
    }

    /// Align the lines vertical to *their parents container*
    pub fn align_children_vertical_in_parent_bounds(&mut self, parent_size: &LayoutSize, vertical_alignment: StyleTextAlignmentVert) {

        let shift_multiplier = match calculate_vertical_shift_multiplier(vertical_alignment) {
            None =>  return,
            Some(s) => s,
        };

        let self_bounds = self.get_bounds();
        let child_bottom_edge = self_bounds.origin.y + self_bounds.size.height;
        let child_top_edge = self_bounds.origin.y;
        let shift = child_top_edge + (parent_size.height - child_bottom_edge);

        for line in self.lines.iter_mut() {
            line.bounds.origin.y += shift * shift_multiplier;
        }
    }
}

#[inline]
pub fn calculate_horizontal_shift_multiplier(horizontal_alignment: StyleTextAlignmentHorz) -> Option<f32> {
    use azul_css::StyleTextAlignmentHorz::*;
    match horizontal_alignment {
        Left => None,
        Center => Some(0.5), // move the line by the half width
        Right => Some(1.0), // move the line by the full width
    }
}

#[inline]
pub fn calculate_vertical_shift_multiplier(vertical_alignment: StyleTextAlignmentVert) -> Option<f32> {
    use azul_css::StyleTextAlignmentVert::*;
    match vertical_alignment {
        Top => None,
        Center => Some(0.5), // move the line by the half width
        Bottom => Some(1.0), // move the line by the full width
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Ord, PartialOrd)]
#[repr(C)]
pub struct ExternalScrollId(pub u64, pub PipelineId);

impl ::std::fmt::Display for ExternalScrollId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExternalScrollId({:0x}, {})", self.0, self.1)
    }
}

impl ::std::fmt::Debug for ExternalScrollId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Default, Debug, Clone)]
pub struct ScrolledNodes {
    pub overflowing_nodes: BTreeMap<NodeId, OverflowingScrollNode>,
    pub tags_to_node_ids: BTreeMap<ScrollTagId, NodeId>,
}

#[derive(Debug, Clone)]
pub struct OverflowingScrollNode {
    pub child_rect: LayoutRect,
    pub parent_external_scroll_id: ExternalScrollId,
    pub parent_dom_hash: DomHash,
    pub scroll_tag_id: ScrollTagId,
}

#[derive(Debug, Default, Clone)]
pub struct LayoutResult {
    pub rects: NodeDataContainer<PositionedRectangle>,
    pub word_cache: BTreeMap<NodeId, Words>,
    pub scaled_words: BTreeMap<NodeId, (ScaledWords, FontInstanceKey)>,
    pub positioned_word_cache: BTreeMap<NodeId, (WordPositions, FontInstanceKey)>,
    pub layouted_glyph_cache: BTreeMap<NodeId, LayoutedGlyphs>,
    pub node_depths: Vec<(usize, NodeId)>,
}

/// Layout options that can impact the flow of word positions
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct TextLayoutOptions {
    /// Font size (in pixels) that this text has been laid out with
    pub font_size_px: PixelValue,
    /// Multiplier for the line height, default to 1.0
    pub line_height: Option<f32>,
    /// Additional spacing between glyphs (in pixels)
    pub letter_spacing: Option<PixelValue>,
    /// Additional spacing between words (in pixels)
    pub word_spacing: Option<PixelValue>,
    /// How many spaces should a tab character emulate
    /// (multiplying value, i.e. `4.0` = one tab = 4 spaces)?
    pub tab_width: Option<f32>,
    /// Maximum width of the text (in pixels) - if the text is set to `overflow:visible`, set this to None.
    pub max_horizontal_width: Option<f32>,
    /// How many pixels of leading does the first line have? Note that this added onto to the holes,
    /// so for effects like `:first-letter`, use a hole instead of a leading.
    pub leading: Option<f32>,
    /// This is more important for inline text layout where items can punch "holes"
    /// into the text flow, for example an image that floats to the right.
    ///
    /// TODO: Currently unused!
    pub holes: Vec<LayoutRect>,
}

/// Same as `TextLayoutOptions`, but with the widths / heights of the `PixelValue`s
/// resolved to regular f32s (because `letter_spacing`, `word_spacing`, etc. may be %-based value)
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct ResolvedTextLayoutOptions {
    /// Font size (in pixels) that this text has been laid out with
    pub font_size_px: f32,
    /// Multiplier for the line height, default to 1.0
    pub line_height: Option<f32>,
    /// Additional spacing between glyphs (in pixels)
    pub letter_spacing: Option<f32>,
    /// Additional spacing between words (in pixels)
    pub word_spacing: Option<f32>,
    /// How many spaces should a tab character emulate
    /// (multiplying value, i.e. `4.0` = one tab = 4 spaces)?
    pub tab_width: Option<f32>,
    /// Maximum width of the text (in pixels) - if the text is set to `overflow:visible`, set this to None.
    pub max_horizontal_width: Option<f32>,
    /// How many pixels of leading does the first line have? Note that this added onto to the holes,
    /// so for effects like `:first-letter`, use a hole instead of a leading.
    pub leading: Option<f32>,
    /// This is more important for inline text layout where items can punch "holes"
    /// into the text flow, for example an image that floats to the right.
    ///
    /// TODO: Currently unused!
    pub holes: Vec<LayoutRect>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct ResolvedOffsets {
    pub top: f32,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
}

impl ResolvedOffsets {
    pub const fn zero() -> Self { Self { top: 0.0, left: 0.0, right: 0.0, bottom: 0.0 } }
    pub fn total_vertical(&self) -> f32 { self.top + self.bottom }
    pub fn total_horizontal(&self) -> f32 { self.left + self.right }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct PositionedRectangle {
    /// Outer bounds of the rectangle
    pub size: LayoutSize,
    /// How the rectangle should be positioned
    pub position: PositionInfo,
    /// Padding of the rectangle
    pub padding: ResolvedOffsets,
    /// Margin of the rectangle
    pub margin: ResolvedOffsets,
    /// Border widths of the rectangle
    pub border_widths: ResolvedOffsets,
    /// If this is an inline rectangle, resolve the %-based font sizes
    /// and store them here.
    pub resolved_text_layout_options: Option<(ResolvedTextLayoutOptions, InlineTextLayout, LayoutRect)>,
    /// Determines if the rect should be clipped or not (TODO: x / y as separate fields!)
    pub overflow: OverflowInfo,
}

impl Default for PositionedRectangle {
    fn default() -> Self {
        PositionedRectangle {
            size: LayoutSize::zero(),
            position: PositionInfo::Static { x_offset: 0.0, y_offset: 0.0, static_x_offset: 0.0, static_y_offset: 0.0 },
            padding: ResolvedOffsets::zero(),
            margin: ResolvedOffsets::zero(),
            border_widths: ResolvedOffsets::zero(),
            resolved_text_layout_options: None,
            overflow: OverflowInfo::default(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct OverflowInfo {
    pub overflow_x: DirectionalOverflowInfo,
    pub overflow_y: DirectionalOverflowInfo,
}

// stores how much the children overflow the parent in the given direction
// if amount is negative, the children do not overflow the parent
// if the amount is set to None, that means there are no children for this node, so no overflow can be calculated
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum DirectionalOverflowInfo {
    Scroll { amount: Option<f32> },
    Auto { amount: Option<f32> },
    Hidden { amount: Option<f32> },
    Visible { amount: Option<f32> },
}

impl Default for DirectionalOverflowInfo {
    fn default() -> DirectionalOverflowInfo {
        DirectionalOverflowInfo::Auto { amount: None }
    }
}

impl DirectionalOverflowInfo {

    #[inline]
    pub fn get_amount(&self) -> Option<f32> {
        match self {
            DirectionalOverflowInfo::Scroll { amount: Some(s) } |
            DirectionalOverflowInfo::Auto { amount: Some(s) } |
            DirectionalOverflowInfo::Hidden { amount: Some(s) } |
            DirectionalOverflowInfo::Visible { amount: Some(s) } => Some(*s),
            _ => None
        }
    }

    #[inline]
    pub fn is_negative(&self) -> bool {
        match self {
            DirectionalOverflowInfo::Scroll { amount: Some(s) } |
            DirectionalOverflowInfo::Auto { amount: Some(s) } |
            DirectionalOverflowInfo::Hidden { amount: Some(s) } |
            DirectionalOverflowInfo::Visible { amount: Some(s) } => { !s.is_sign_positive() },
            _ => true // no overflow = no scrollbar
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        match self {
            DirectionalOverflowInfo::Scroll { amount: None } |
            DirectionalOverflowInfo::Auto { amount: None } |
            DirectionalOverflowInfo::Hidden { amount: None } |
            DirectionalOverflowInfo::Visible { amount: None } => true,
            _ => false
        }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum PositionInfo {
    Static { x_offset: f32, y_offset: f32, static_x_offset: f32, static_y_offset: f32 },
    Fixed { x_offset: f32, y_offset: f32, static_x_offset: f32, static_y_offset: f32 },
    Absolute { x_offset: f32, y_offset: f32, static_x_offset: f32, static_y_offset: f32 },
    Relative { x_offset: f32, y_offset: f32, static_x_offset: f32, static_y_offset: f32 },
}

impl PositionInfo {
    pub fn is_positioned(&self) -> bool {
        match self {
            PositionInfo::Static { .. } => false,
            PositionInfo::Fixed { .. } => true,
            PositionInfo::Absolute { .. } => true,
            PositionInfo::Relative { .. } => true,
        }
    }
}
impl PositionedRectangle {

    pub fn get_static_bounds(&self) -> Option<LayoutRect> {
        match self.position {
            PositionInfo::Static { static_x_offset, static_y_offset, .. }     => Some(LayoutRect::new(
                LayoutPoint::new(static_x_offset, static_y_offset), self.size
            )),
            PositionInfo::Fixed { .. }      => None,
            PositionInfo::Absolute { .. }   => None, // TODO?
            PositionInfo::Relative { static_x_offset, static_y_offset, .. }   => Some(LayoutRect::new(
                LayoutPoint::new(static_x_offset, static_y_offset), self.size
            )),
        }
    }

    pub fn to_layouted_rectangle(&self) -> LayoutedRectangle {
        LayoutedRectangle {
            size: self.size,
            position: self.position,
            padding: self.padding,
            margin: self.margin,
            border_widths: self.border_widths,
            overflow: self.overflow,
        }
    }

    // Returns the rect where the content should be placed (for example the text itself)
    pub fn get_content_size(&self) -> LayoutSize {
        self.size
    }

    // Returns the rect that includes bounds, expanded by the padding + the border widths
    pub fn get_background_bounds(&self) -> (LayoutSize, PositionInfo) {

        use crate::ui_solver::PositionInfo::*;

        let b_size = LayoutSize {
            width: self.size.width + self.padding.total_horizontal() + self.border_widths.total_horizontal(),
            height: self.size.height + self.padding.total_vertical() + self.border_widths.total_vertical(),
        };

        let x_offset_add = 0.0 - self.padding.left - self.border_widths.left;
        let y_offset_add = 0.0 - self.padding.top - self.border_widths.top;

        let b_position = match self.position {
            Static { x_offset, y_offset, static_x_offset, static_y_offset } => Static { x_offset: x_offset + x_offset_add, y_offset: y_offset + y_offset_add, static_x_offset, static_y_offset },
            Fixed { x_offset, y_offset, static_x_offset, static_y_offset } => Fixed { x_offset: x_offset + x_offset_add, y_offset: y_offset + y_offset_add, static_x_offset, static_y_offset },
            Relative { x_offset, y_offset, static_x_offset, static_y_offset } => Relative { x_offset: x_offset + x_offset_add, y_offset: y_offset + y_offset_add, static_x_offset, static_y_offset },
            Absolute { x_offset, y_offset, static_x_offset, static_y_offset } => Absolute { x_offset: x_offset + x_offset_add, y_offset: y_offset + y_offset_add, static_x_offset, static_y_offset },
        };

        (b_size, b_position)
    }

    pub fn get_margin_box_width(&self) -> f32 {
        self.size.width +
        self.padding.total_horizontal() +
        self.border_widths.total_horizontal() +
        self.margin.total_horizontal()
    }

    pub fn get_margin_box_height(&self) -> f32 {
        self.size.height +
        self.padding.total_vertical() +
        self.border_widths.total_vertical() +
        self.margin.total_vertical()
    }

    pub fn get_left_leading(&self) -> f32 {
        self.margin.left +
        self.padding.left +
        self.border_widths.left
    }

    pub fn get_top_leading(&self) -> f32 {
        self.margin.top +
        self.padding.top +
        self.border_widths.top
    }
}

/// Same as `PositionedRectangle`, but without the `text_layout_options`,
/// so that the struct implements `Copy`.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct LayoutedRectangle {
    /// Outer bounds of the rectangle
    pub size: LayoutSize,
    /// How the rectangle should be positioned
    pub position: PositionInfo,
    /// Padding of the rectangle
    pub padding: ResolvedOffsets,
    /// Margin of the rectangle
    pub margin: ResolvedOffsets,
    /// Border widths of the rectangle
    pub border_widths: ResolvedOffsets,
    /// Determines if the rect should be clipped or not (TODO: x / y as separate fields!)
    pub overflow: OverflowInfo,
}