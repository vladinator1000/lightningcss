//! CSS selectors.

use crate::compat::Feature;
use crate::error::{ParserError, PrinterError};
use crate::printer::Printer;
use crate::properties::custom::TokenList;
use crate::rules::StyleContext;
use crate::stylesheet::{ParserOptions, PrinterOptions};
use crate::targets::Browsers;
use crate::traits::{Parse, ParseWithOptions, ToCss};
use crate::values::ident::Ident;
use crate::values::string::CSSString;
use crate::vendor_prefix::VendorPrefix;
#[cfg(feature = "visitor")]
use crate::visitor::{Visit, VisitTypes, Visitor};
use crate::{macros::enum_property, values::string::CowArcStr};
use cssparser::*;
use parcel_selectors::parser::SelectorParseErrorKind;
use parcel_selectors::{
  attr::{AttrSelectorOperator, ParsedAttrSelectorOperation, ParsedCaseSensitivity},
  parser::SelectorImpl,
};
use std::collections::HashSet;
use std::fmt;

#[cfg(feature = "serde")]
use crate::serialization::*;

mod private {
  #[derive(Debug, Clone, PartialEq, Eq)]
  #[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
  pub struct Selectors;
}

use private::Selectors;

/// A list of selectors.
pub type SelectorList<'i> = parcel_selectors::SelectorList<'i, Selectors>;
/// A CSS selector, including a list of components.
pub type Selector<'i> = parcel_selectors::parser::Selector<'i, Selectors>;
/// An individual component within a selector.
pub type Component<'i> = parcel_selectors::parser::Component<'i, Selectors>;
/// A combinator.
pub use parcel_selectors::parser::Combinator;

impl<'i> SelectorImpl<'i> for Selectors {
  type AttrValue = CSSString<'i>;
  type Identifier = Ident<'i>;
  type LocalName = Ident<'i>;
  type NamespacePrefix = Ident<'i>;
  type NamespaceUrl = CowArcStr<'i>;
  type BorrowedNamespaceUrl = CowArcStr<'i>;
  type BorrowedLocalName = Ident<'i>;

  type NonTSPseudoClass = PseudoClass<'i>;
  type PseudoElement = PseudoElement<'i>;
  type VendorPrefix = VendorPrefix;

  type ExtraMatchingData = ();

  fn to_css<W: fmt::Write>(selectors: &SelectorList<'i>, dest: &mut W) -> std::fmt::Result {
    let mut printer = Printer::new(dest, PrinterOptions::default());
    serialize_selector_list(selectors.0.iter(), &mut printer, None, false).map_err(|_| std::fmt::Error)
  }
}

pub(crate) struct SelectorParser<'a, 'o, 'i> {
  pub is_nesting_allowed: bool,
  pub options: &'a ParserOptions<'o, 'i>,
}

impl<'a, 'o, 'i> parcel_selectors::parser::Parser<'i> for SelectorParser<'a, 'o, 'i> {
  type Impl = Selectors;
  type Error = ParserError<'i>;

  fn parse_non_ts_pseudo_class(
    &self,
    loc: SourceLocation,
    name: CowRcStr<'i>,
  ) -> Result<PseudoClass<'i>, ParseError<'i, Self::Error>> {
    use PseudoClass::*;
    let pseudo_class = match_ignore_ascii_case! { &name,
      // https://drafts.csswg.org/selectors-4/#useraction-pseudos
      "hover" => Hover,
      "active" => Active,
      "focus" => Focus,
      "focus-visible" => FocusVisible,
      "focus-within" => FocusWithin,

      // https://drafts.csswg.org/selectors-4/#time-pseudos
      "current" => Current,
      "past" => Past,
      "future" => Future,

      // https://drafts.csswg.org/selectors-4/#resource-pseudos
      "playing" => Playing,
      "paused" => Paused,
      "seeking" => Seeking,
      "buffering" => Buffering,
      "stalled" => Stalled,
      "muted" => Muted,
      "volume-locked" => VolumeLocked,

      // https://fullscreen.spec.whatwg.org/#:fullscreen-pseudo-class
      "fullscreen" => Fullscreen(VendorPrefix::None),
      "-webkit-full-screen" => Fullscreen(VendorPrefix::WebKit),
      "-moz-full-screen" => Fullscreen(VendorPrefix::Moz),
      "-ms-fullscreen" => Fullscreen(VendorPrefix::Ms),

      // https://drafts.csswg.org/selectors-4/#the-defined-pseudo
      "defined" => Defined,

      // https://drafts.csswg.org/selectors-4/#location
      "any-link" => AnyLink(VendorPrefix::None),
      "-webkit-any-link" => AnyLink(VendorPrefix::WebKit),
      "-moz-any-link" => AnyLink(VendorPrefix::Moz),
      "link" => Link,
      "local-link" => LocalLink,
      "target" => Target,
      "target-within" => TargetWithin,
      "visited" => Visited,

      // https://drafts.csswg.org/selectors-4/#input-pseudos
      "enabled" => Enabled,
      "disabled" => Disabled,
      "read-only" => ReadOnly(VendorPrefix::None),
      "-moz-read-only" => ReadOnly(VendorPrefix::Moz),
      "read-write" => ReadWrite(VendorPrefix::None),
      "-moz-read-write" => ReadWrite(VendorPrefix::Moz),
      "placeholder-shown" => PlaceholderShown(VendorPrefix::None),
      "-moz-placeholder-shown" => PlaceholderShown(VendorPrefix::Moz),
      "-ms-placeholder-shown" => PlaceholderShown(VendorPrefix::Ms),
      "default" => Default,
      "checked" => Checked,
      "indeterminate" => Indeterminate,
      "blank" => Blank,
      "valid" => Valid,
      "invalid" => Invalid,
      "in-range" => InRange,
      "out-of-range" => OutOfRange,
      "required" => Required,
      "optional" => Optional,
      "user-valid" => UserValid,
      "user-invalid" => UserInvalid,

      // https://html.spec.whatwg.org/multipage/semantics-other.html#selector-autofill
      "autofill" => Autofill(VendorPrefix::None),
      "-webkit-autofill" => Autofill(VendorPrefix::WebKit),
      "-o-autofill" => Autofill(VendorPrefix::O),

      // https://webkit.org/blog/363/styling-scrollbars/
      "horizontal" => WebKitScrollbar(WebKitScrollbarPseudoClass::Horizontal),
      "vertical" => WebKitScrollbar(WebKitScrollbarPseudoClass::Vertical),
      "decrement" => WebKitScrollbar(WebKitScrollbarPseudoClass::Decrement),
      "increment" => WebKitScrollbar(WebKitScrollbarPseudoClass::Increment),
      "start" => WebKitScrollbar(WebKitScrollbarPseudoClass::Start),
      "end" => WebKitScrollbar(WebKitScrollbarPseudoClass::End),
      "double-button" => WebKitScrollbar(WebKitScrollbarPseudoClass::DoubleButton),
      "single-button" => WebKitScrollbar(WebKitScrollbarPseudoClass::SingleButton),
      "no-button" => WebKitScrollbar(WebKitScrollbarPseudoClass::NoButton),
      "corner-present" => WebKitScrollbar(WebKitScrollbarPseudoClass::CornerPresent),
      "window-inactive" => WebKitScrollbar(WebKitScrollbarPseudoClass::WindowInactive),

      _ => {
        if !name.starts_with('-') {
          self.options.warn(loc.new_custom_error(SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name.clone())));
        }
        Custom { name: name.into() }
      }
    };

    Ok(pseudo_class)
  }

  fn parse_non_ts_functional_pseudo_class<'t>(
    &self,
    name: CowRcStr<'i>,
    parser: &mut cssparser::Parser<'i, 't>,
  ) -> Result<PseudoClass<'i>, ParseError<'i, Self::Error>> {
    use PseudoClass::*;
    let pseudo_class = match_ignore_ascii_case! { &name,
      "lang" => {
        let languages = parser.parse_comma_separated(|parser| {
          parser.expect_ident_or_string()
            .map(|s| s.into())
            .map_err(|e| e.into())
        })?;
        Lang { languages }
      },
      "dir" => Dir { direction: Direction::parse(parser)? },
      "local" if self.options.css_modules.is_some() => Local { selector: Box::new(Selector::parse(self, parser)?) },
      "global" if self.options.css_modules.is_some() => Global { selector: Box::new(Selector::parse(self, parser)?) },
      _ => {
        if !name.starts_with('-') {
          self.options.warn(parser.new_custom_error(SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name.clone())));
        }
        CustomFunction {
          name: name.into(),
          arguments: TokenList::parse(parser, &self.options, 0)?
        }
      },
    };

    Ok(pseudo_class)
  }

  fn parse_any_prefix<'t>(&self, name: &str) -> Option<VendorPrefix> {
    match_ignore_ascii_case! { &name,
      "-webkit-any" => Some(VendorPrefix::WebKit),
      "-moz-any" => Some(VendorPrefix::Moz),
      _ => None
    }
  }

  fn parse_pseudo_element(
    &self,
    loc: SourceLocation,
    name: CowRcStr<'i>,
  ) -> Result<PseudoElement<'i>, ParseError<'i, Self::Error>> {
    use PseudoElement::*;
    let pseudo_element = match_ignore_ascii_case! { &name,
      "before" => Before,
      "after" => After,
      "first-line" => FirstLine,
      "first-letter" => FirstLetter,
      "cue" => Cue,
      "cue-region" => CueRegion,
      "selection" => Selection(VendorPrefix::None),
      "-moz-selection" => Selection(VendorPrefix::Moz),
      "placeholder" => Placeholder(VendorPrefix::None),
      "-webkit-input-placeholder" => Placeholder(VendorPrefix::WebKit),
      "-moz-placeholder" => Placeholder(VendorPrefix::Moz),
      "-ms-input-placeholder" => Placeholder(VendorPrefix::Moz),
      "marker" => Marker,
      "backdrop" => Backdrop(VendorPrefix::None),
      "-webkit-backdrop" => Backdrop(VendorPrefix::WebKit),
      "file-selector-button" => FileSelectorButton(VendorPrefix::None),
      "-webkit-file-upload-button" => FileSelectorButton(VendorPrefix::WebKit),
      "-ms-browse" => FileSelectorButton(VendorPrefix::Ms),

      "-webkit-scrollbar" => WebKitScrollbar(WebKitScrollbarPseudoElement::Scrollbar),
      "-webkit-scrollbar-button" => WebKitScrollbar(WebKitScrollbarPseudoElement::Button),
      "-webkit-scrollbar-track" => WebKitScrollbar(WebKitScrollbarPseudoElement::Track),
      "-webkit-scrollbar-track-piece" => WebKitScrollbar(WebKitScrollbarPseudoElement::TrackPiece),
      "-webkit-scrollbar-thumb" => WebKitScrollbar(WebKitScrollbarPseudoElement::Thumb),
      "-webkit-scrollbar-corner" => WebKitScrollbar(WebKitScrollbarPseudoElement::Corner),
      "-webkit-resizer" => WebKitScrollbar(WebKitScrollbarPseudoElement::Resizer),

      _ => {
        if !name.starts_with('-') {
          self.options.warn(loc.new_custom_error(SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name.clone())));
        }
        Custom { name: name.into() }
      }
    };

    Ok(pseudo_element)
  }

  fn parse_functional_pseudo_element<'t>(
    &self,
    name: CowRcStr<'i>,
    arguments: &mut Parser<'i, 't>,
  ) -> Result<<Self::Impl as SelectorImpl<'i>>::PseudoElement, ParseError<'i, Self::Error>> {
    use PseudoElement::*;
    let pseudo_element = match_ignore_ascii_case! { &name,
      "cue" => CueFunction { selector: Box::new(Selector::parse(self, arguments)?) },
      "cue-region" => CueRegionFunction { selector: Box::new(Selector::parse(self, arguments)?) },
      _ => {
        if !name.starts_with('-') {
          self.options.warn(arguments.new_custom_error(SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name.clone())));
        }
        CustomFunction { name: name.into(), arguments: TokenList::parse(arguments, &self.options, 0)? }
      }
    };

    Ok(pseudo_element)
  }

  #[inline]
  fn parse_slotted(&self) -> bool {
    true
  }

  #[inline]
  fn parse_host(&self) -> bool {
    true
  }

  #[inline]
  fn parse_is_and_where(&self) -> bool {
    true
  }

  #[inline]
  fn parse_part(&self) -> bool {
    true
  }

  fn default_namespace(&self) -> Option<CowArcStr<'i>> {
    None
  }

  fn namespace_for_prefix(&self, prefix: &Ident<'i>) -> Option<CowArcStr<'i>> {
    Some(prefix.0.clone())
  }

  #[inline]
  fn is_nesting_allowed(&self) -> bool {
    self.is_nesting_allowed
  }
}

enum_property! {
  /// The [:dir()](https://drafts.csswg.org/selectors-4/#the-dir-pseudo) pseudo class.
  #[derive(Eq)]
  pub enum Direction {
    /// Left to right
    Ltr,
    /// Right to left
    Rtl,
  }
}

/// A pseudo class.
#[derive(Clone, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "kind", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum PseudoClass<'i> {
  // https://drafts.csswg.org/selectors-4/#linguistic-pseudos
  /// The [:lang()](https://drafts.csswg.org/selectors-4/#the-lang-pseudo) pseudo class.
  Lang {
    /// A list of language codes.
    #[cfg_attr(feature = "serde", serde(borrow))]
    languages: Vec<CowArcStr<'i>>,
  },
  /// The [:dir()](https://drafts.csswg.org/selectors-4/#the-dir-pseudo) pseudo class.
  Dir {
    /// A direction.
    direction: Direction,
  },

  // https://drafts.csswg.org/selectors-4/#useraction-pseudos
  /// The [:hover](https://drafts.csswg.org/selectors-4/#the-hover-pseudo) pseudo class.
  Hover,
  /// The [:active](https://drafts.csswg.org/selectors-4/#the-active-pseudo) pseudo class.
  Active,
  /// The [:focus](https://drafts.csswg.org/selectors-4/#the-focus-pseudo) pseudo class.
  Focus,
  /// The [:focus-visible](https://drafts.csswg.org/selectors-4/#the-focus-visible-pseudo) pseudo class.
  FocusVisible,
  /// The [:focus-within](https://drafts.csswg.org/selectors-4/#the-focus-within-pseudo) pseudo class.
  FocusWithin,

  // https://drafts.csswg.org/selectors-4/#time-pseudos
  /// The [:current](https://drafts.csswg.org/selectors-4/#the-current-pseudo) pseudo class.
  Current,
  /// The [:past](https://drafts.csswg.org/selectors-4/#the-past-pseudo) pseudo class.
  Past,
  /// The [:future](https://drafts.csswg.org/selectors-4/#the-future-pseudo) pseudo class.
  Future,

  // https://drafts.csswg.org/selectors-4/#resource-pseudos
  /// The [:playing](https://drafts.csswg.org/selectors-4/#selectordef-playing) pseudo class.
  Playing,
  /// The [:paused](https://drafts.csswg.org/selectors-4/#selectordef-paused) pseudo class.
  Paused,
  /// The [:seeking](https://drafts.csswg.org/selectors-4/#selectordef-seeking) pseudo class.
  Seeking,
  /// The [:buffering](https://drafts.csswg.org/selectors-4/#selectordef-buffering) pseudo class.
  Buffering,
  /// The [:stalled](https://drafts.csswg.org/selectors-4/#selectordef-stalled) pseudo class.
  Stalled,
  /// The [:muted](https://drafts.csswg.org/selectors-4/#selectordef-muted) pseudo class.
  Muted,
  /// The [:volume-locked](https://drafts.csswg.org/selectors-4/#selectordef-volume-locked) pseudo class.
  VolumeLocked,

  /// The [:fullscreen](https://fullscreen.spec.whatwg.org/#:fullscreen-pseudo-class) pseudo class.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Fullscreen(VendorPrefix),

  /// The [:defined](https://drafts.csswg.org/selectors-4/#the-defined-pseudo) pseudo class.
  Defined,

  // https://drafts.csswg.org/selectors-4/#location
  /// The [:any-link](https://drafts.csswg.org/selectors-4/#the-any-link-pseudo) pseudo class.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  AnyLink(VendorPrefix),
  /// The [:link](https://drafts.csswg.org/selectors-4/#link-pseudo) pseudo class.
  Link,
  /// The [:local-link](https://drafts.csswg.org/selectors-4/#the-local-link-pseudo) pseudo class.
  LocalLink,
  /// The [:target](https://drafts.csswg.org/selectors-4/#the-target-pseudo) pseudo class.
  Target,
  /// The [:target-within](https://drafts.csswg.org/selectors-4/#the-target-within-pseudo) pseudo class.
  TargetWithin,
  /// The [:visited](https://drafts.csswg.org/selectors-4/#visited-pseudo) pseudo class.
  Visited,

  // https://drafts.csswg.org/selectors-4/#input-pseudos
  /// The [:enabled](https://drafts.csswg.org/selectors-4/#enabled-pseudo) pseudo class.
  Enabled,
  /// The [:disabled](https://drafts.csswg.org/selectors-4/#disabled-pseudo) pseudo class.
  Disabled,
  /// The [:read-only](https://drafts.csswg.org/selectors-4/#read-only-pseudo) pseudo class.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  ReadOnly(VendorPrefix),
  /// The [:read-write](https://drafts.csswg.org/selectors-4/#read-write-pseudo) pseudo class.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  ReadWrite(VendorPrefix),
  /// The [:placeholder-shown](https://drafts.csswg.org/selectors-4/#placeholder) pseudo class.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  PlaceholderShown(VendorPrefix),
  /// The [:default](https://drafts.csswg.org/selectors-4/#the-default-pseudo) pseudo class.
  Default,
  /// The [:checked](https://drafts.csswg.org/selectors-4/#checked) pseudo class.
  Checked,
  /// The [:indeterminate](https://drafts.csswg.org/selectors-4/#indeterminate) pseudo class.
  Indeterminate,
  /// The [:blank](https://drafts.csswg.org/selectors-4/#blank) pseudo class.
  Blank,
  /// The [:valid](https://drafts.csswg.org/selectors-4/#valid-pseudo) pseudo class.
  Valid,
  /// The [:invalid](https://drafts.csswg.org/selectors-4/#invalid-pseudo) pseudo class.
  Invalid,
  /// The [:in-range](https://drafts.csswg.org/selectors-4/#in-range-pseudo) pseudo class.
  InRange,
  /// The [:out-of-range](https://drafts.csswg.org/selectors-4/#out-of-range-pseudo) pseudo class.
  OutOfRange,
  /// The [:required](https://drafts.csswg.org/selectors-4/#required-pseudo) pseudo class.
  Required,
  /// The [:optional](https://drafts.csswg.org/selectors-4/#optional-pseudo) pseudo class.
  Optional,
  /// The [:user-valid](https://drafts.csswg.org/selectors-4/#user-valid-pseudo) pseudo class.
  UserValid,
  /// The [:used-invalid](https://drafts.csswg.org/selectors-4/#user-invalid-pseudo) pseudo class.
  UserInvalid,

  /// The [:autofill](https://html.spec.whatwg.org/multipage/semantics-other.html#selector-autofill) pseudo class.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Autofill(VendorPrefix),

  // CSS modules
  /// The CSS modules :local() pseudo class.
  Local {
    /// A local selector.
    selector: Box<Selector<'i>>,
  },
  /// The CSS modules :global() pseudo class.
  Global {
    /// A global selector.
    selector: Box<Selector<'i>>,
  },

  /// A [webkit scrollbar](https://webkit.org/blog/363/styling-scrollbars/) pseudo class.
  // https://webkit.org/blog/363/styling-scrollbars/
  #[cfg_attr(
    feature = "serde",
    serde(rename = "webkit-scrollbar", with = "ValueWrapper::<WebKitScrollbarPseudoClass>")
  )]
  WebKitScrollbar(WebKitScrollbarPseudoClass),
  /// An unknown pseudo class.
  Custom {
    /// The pseudo class name.
    name: CowArcStr<'i>,
  },
  /// An unknown functional pseudo class.
  CustomFunction {
    /// The pseudo class name.
    name: CowArcStr<'i>,
    /// The arguments of the pseudo class function.
    arguments: TokenList<'i>,
  },
}

/// A [webkit scrollbar](https://webkit.org/blog/363/styling-scrollbars/) pseudo class.
#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum WebKitScrollbarPseudoClass {
  /// :horizontal
  Horizontal,
  /// :vertical
  Vertical,
  /// :decrement
  Decrement,
  /// :increment
  Increment,
  /// :start
  Start,
  /// :end
  End,
  /// :double-button
  DoubleButton,
  /// :single-button
  SingleButton,
  /// :no-button
  NoButton,
  /// :corner-present
  CornerPresent,
  /// :window-inactive
  WindowInactive,
}

impl<'i> parcel_selectors::parser::NonTSPseudoClass<'i> for PseudoClass<'i> {
  type Impl = Selectors;

  fn is_active_or_hover(&self) -> bool {
    matches!(*self, PseudoClass::Active | PseudoClass::Hover)
  }

  fn is_user_action_state(&self) -> bool {
    matches!(
      *self,
      PseudoClass::Active
        | PseudoClass::Hover
        | PseudoClass::Focus
        | PseudoClass::FocusWithin
        | PseudoClass::FocusVisible
    )
  }

  fn is_valid_before_webkit_scrollbar(&self) -> bool {
    !matches!(*self, PseudoClass::WebKitScrollbar(..))
  }

  fn is_valid_after_webkit_scrollbar(&self) -> bool {
    // https://github.com/WebKit/WebKit/blob/02fbf9b7aa435edb96cbf563a8d4dcf1aa73b4b3/Source/WebCore/css/parser/CSSSelectorParser.cpp#L285
    matches!(
      *self,
      PseudoClass::WebKitScrollbar(..)
        | PseudoClass::Enabled
        | PseudoClass::Disabled
        | PseudoClass::Hover
        | PseudoClass::Active
    )
  }
}

impl<'i> cssparser::ToCss for PseudoClass<'i> {
  fn to_css<W>(&self, _: &mut W) -> std::fmt::Result
  where
    W: fmt::Write,
  {
    unreachable!()
  }
}

fn serialize_pseudo_class<'a, 'i, W>(
  pseudo_class: &PseudoClass<'i>,
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
) -> Result<(), PrinterError>
where
  W: fmt::Write,
{
  use PseudoClass::*;
  match pseudo_class {
    Lang { languages: lang } => {
      dest.write_str(":lang(")?;
      let mut first = true;
      for lang in lang {
        if first {
          first = false;
        } else {
          dest.delim(',', false)?;
        }
        serialize_identifier(lang, dest)?;
      }
      return dest.write_str(")");
    }
    Dir { direction: dir } => {
      dest.write_str(":dir(")?;
      dir.to_css(dest)?;
      return dest.write_str(")");
    }
    _ => {}
  }

  macro_rules! write_prefixed {
    ($prefix: ident, $val: expr) => {{
      dest.write_char(':')?;
      // If the printer has a vendor prefix override, use that.
      let vp = if !dest.vendor_prefix.is_empty() {
        dest.vendor_prefix
      } else {
        *$prefix
      };
      vp.to_css(dest)?;
      dest.write_str($val)
    }};
  }

  macro_rules! pseudo {
    ($key: ident, $s: literal) => {{
      let class = if let Some(pseudo_classes) = &dest.pseudo_classes {
        pseudo_classes.$key
      } else {
        None
      };

      if let Some(class) = class {
        dest.write_char('.')?;
        dest.write_ident(class)
      } else {
        dest.write_str($s)
      }
    }};
  }

  match pseudo_class {
    // https://drafts.csswg.org/selectors-4/#useraction-pseudos
    Hover => pseudo!(hover, ":hover"),
    Active => pseudo!(active, ":active"),
    Focus => pseudo!(focus, ":focus"),
    FocusVisible => pseudo!(focus_visible, ":focus-visible"),
    FocusWithin => pseudo!(focus_within, ":focus-within"),

    // https://drafts.csswg.org/selectors-4/#time-pseudos
    Current => dest.write_str(":current"),
    Past => dest.write_str(":past"),
    Future => dest.write_str(":future"),

    // https://drafts.csswg.org/selectors-4/#resource-pseudos
    Playing => dest.write_str(":playing"),
    Paused => dest.write_str(":paused"),
    Seeking => dest.write_str(":seeking"),
    Buffering => dest.write_str(":buffering"),
    Stalled => dest.write_str(":stalled"),
    Muted => dest.write_str(":muted"),
    VolumeLocked => dest.write_str(":volume-locked"),

    // https://fullscreen.spec.whatwg.org/#:fullscreen-pseudo-class
    Fullscreen(prefix) => {
      dest.write_char(':')?;
      let vp = if !dest.vendor_prefix.is_empty() {
        dest.vendor_prefix
      } else {
        *prefix
      };
      vp.to_css(dest)?;
      if vp == VendorPrefix::WebKit || vp == VendorPrefix::Moz {
        dest.write_str("full-screen")
      } else {
        dest.write_str("fullscreen")
      }
    }

    // https://drafts.csswg.org/selectors-4/#the-defined-pseudo
    Defined => dest.write_str(":defined"),

    // https://drafts.csswg.org/selectors-4/#location
    AnyLink(prefix) => write_prefixed!(prefix, "any-link"),
    Link => dest.write_str(":link"),
    LocalLink => dest.write_str(":local-link"),
    Target => dest.write_str(":target"),
    TargetWithin => dest.write_str(":target-within"),
    Visited => dest.write_str(":visited"),

    // https://drafts.csswg.org/selectors-4/#input-pseudos
    Enabled => dest.write_str(":enabled"),
    Disabled => dest.write_str(":disabled"),
    ReadOnly(prefix) => write_prefixed!(prefix, "read-only"),
    ReadWrite(prefix) => write_prefixed!(prefix, "read-write"),
    PlaceholderShown(prefix) => write_prefixed!(prefix, "placeholder-shown"),
    Default => dest.write_str(":default"),
    Checked => dest.write_str(":checked"),
    Indeterminate => dest.write_str(":indeterminate"),
    Blank => dest.write_str(":blank"),
    Valid => dest.write_str(":valid"),
    Invalid => dest.write_str(":invalid"),
    InRange => dest.write_str(":in-range"),
    OutOfRange => dest.write_str(":out-of-range"),
    Required => dest.write_str(":required"),
    Optional => dest.write_str(":optional"),
    UserValid => dest.write_str(":user-valid"),
    UserInvalid => dest.write_str(":user-invalid"),

    // https://html.spec.whatwg.org/multipage/semantics-other.html#selector-autofill
    Autofill(prefix) => write_prefixed!(prefix, "autofill"),

    Local { selector } => serialize_selector(selector, dest, context, false),
    Global { selector } => {
      let css_module = std::mem::take(&mut dest.css_module);
      serialize_selector(selector, dest, context, false)?;
      dest.css_module = css_module;
      Ok(())
    }

    // https://webkit.org/blog/363/styling-scrollbars/
    WebKitScrollbar(s) => {
      use WebKitScrollbarPseudoClass::*;
      dest.write_str(match s {
        Horizontal => ":horizontal",
        Vertical => ":vertical",
        Decrement => ":decrement",
        Increment => ":increment",
        Start => ":start",
        End => ":end",
        DoubleButton => ":double-button",
        SingleButton => ":single-button",
        NoButton => ":no-button",
        CornerPresent => ":corner-present",
        WindowInactive => ":window-inactive",
      })
    }

    Lang { languages: _ } | Dir { direction: _ } => unreachable!(),
    Custom { name } => {
      dest.write_char(':')?;
      return dest.write_str(&name);
    }
    CustomFunction { name, arguments: args } => {
      dest.write_char(':')?;
      dest.write_str(name)?;
      dest.write_char('(')?;
      args.to_css(dest, false)?;
      dest.write_char(')')
    }
  }
}

impl<'i> PseudoClass<'i> {
  pub(crate) fn is_equivalent(&self, other: &PseudoClass<'i>) -> bool {
    use PseudoClass::*;
    match (self, other) {
      (Fullscreen(_), Fullscreen(_))
      | (AnyLink(_), AnyLink(_))
      | (ReadOnly(_), ReadOnly(_))
      | (ReadWrite(_), ReadWrite(_))
      | (PlaceholderShown(_), PlaceholderShown(_))
      | (Autofill(_), Autofill(_)) => true,
      (a, b) => a == b,
    }
  }

  pub(crate) fn get_prefix(&self) -> VendorPrefix {
    use PseudoClass::*;
    match self {
      Fullscreen(p) | AnyLink(p) | ReadOnly(p) | ReadWrite(p) | PlaceholderShown(p) | Autofill(p) => *p,
      _ => VendorPrefix::empty(),
    }
  }

  pub(crate) fn get_necessary_prefixes(&self, targets: Browsers) -> VendorPrefix {
    use crate::prefixes::Feature;
    use PseudoClass::*;
    let feature = match self {
      Fullscreen(p) if *p == VendorPrefix::None => Feature::PseudoClassFullscreen,
      AnyLink(p) if *p == VendorPrefix::None => Feature::PseudoClassAnyLink,
      ReadOnly(p) if *p == VendorPrefix::None => Feature::PseudoClassReadOnly,
      ReadWrite(p) if *p == VendorPrefix::None => Feature::PseudoClassReadWrite,
      PlaceholderShown(p) if *p == VendorPrefix::None => Feature::PseudoClassPlaceholderShown,
      Autofill(p) if *p == VendorPrefix::None => Feature::PseudoClassAutofill,
      _ => return VendorPrefix::empty(),
    };

    feature.prefixes_for(targets)
  }
}

/// A pseudo element.
#[derive(PartialEq, Clone, Debug)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "kind", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum PseudoElement<'i> {
  /// The [::after](https://drafts.csswg.org/css-pseudo-4/#selectordef-after) pseudo element.
  After,
  /// The [::before](https://drafts.csswg.org/css-pseudo-4/#selectordef-before) pseudo element.
  Before,
  /// The [::first-line](https://drafts.csswg.org/css-pseudo-4/#first-line-pseudo) pseudo element.
  FirstLine,
  /// The [::first-letter](https://drafts.csswg.org/css-pseudo-4/#first-letter-pseudo) pseudo element.
  FirstLetter,
  /// The [::selection](https://drafts.csswg.org/css-pseudo-4/#selectordef-selection) pseudo element.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Selection(VendorPrefix),
  /// The [::placeholder](https://drafts.csswg.org/css-pseudo-4/#placeholder-pseudo) pseudo element.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Placeholder(VendorPrefix),
  ///  The [::marker](https://drafts.csswg.org/css-pseudo-4/#marker-pseudo) pseudo element.
  Marker,
  /// The [::backdrop](https://fullscreen.spec.whatwg.org/#::backdrop-pseudo-element) pseudo element.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Backdrop(VendorPrefix),
  /// The [::file-selector-button](https://drafts.csswg.org/css-pseudo-4/#file-selector-button-pseudo) pseudo element.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  FileSelectorButton(VendorPrefix),
  /// A [webkit scrollbar](https://webkit.org/blog/363/styling-scrollbars/) pseudo element.
  #[cfg_attr(
    feature = "serde",
    serde(rename = "webkit-scrollbar", with = "ValueWrapper::<WebKitScrollbarPseudoElement>")
  )]
  WebKitScrollbar(WebKitScrollbarPseudoElement),
  /// The [::cue](https://w3c.github.io/webvtt/#the-cue-pseudo-element) pseudo element.
  Cue,
  /// The [::cue-region](https://w3c.github.io/webvtt/#the-cue-region-pseudo-element) pseudo element.
  CueRegion,
  /// The [::cue()](https://w3c.github.io/webvtt/#cue-selector) functional pseudo element.
  CueFunction {
    /// The selector argument.
    selector: Box<Selector<'i>>,
  },
  /// The [::cue-region()](https://w3c.github.io/webvtt/#cue-region-selector) functional pseudo element.
  CueRegionFunction {
    /// The selector argument.
    selector: Box<Selector<'i>>,
  },
  /// An unknown pseudo element.
  Custom {
    /// The name of the pseudo element.
    #[cfg_attr(feature = "serde", serde(borrow))]
    name: CowArcStr<'i>,
  },
  /// An unknown functional pseudo element.
  CustomFunction {
    ///The name of the pseudo element.
    name: CowArcStr<'i>,
    /// The arguments of the pseudo element function.
    arguments: TokenList<'i>,
  },
}

/// A [webkit scrollbar](https://webkit.org/blog/363/styling-scrollbars/) pseudo element.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum WebKitScrollbarPseudoElement {
  /// ::-webkit-scrollbar
  Scrollbar,
  /// ::-webkit-scrollbar-button
  Button,
  /// ::-webkit-scrollbar-track
  Track,
  /// ::-webkit-scrollbar-track-piece
  TrackPiece,
  /// ::-webkit-scrollbar-thumb
  Thumb,
  /// ::-webkit-scrollbar-corner
  Corner,
  /// ::-webkit-resizer
  Resizer,
}

impl<'i> cssparser::ToCss for PseudoElement<'i> {
  fn to_css<W>(&self, _: &mut W) -> std::fmt::Result
  where
    W: fmt::Write,
  {
    unreachable!();
  }
}

fn serialize_pseudo_element<'a, 'i, W>(
  pseudo_element: &PseudoElement,
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
) -> Result<(), PrinterError>
where
  W: fmt::Write,
{
  use PseudoElement::*;

  macro_rules! write_prefix {
    ($prefix: ident) => {{
      dest.write_str("::")?;
      // If the printer has a vendor prefix override, use that.
      let vp = if !dest.vendor_prefix.is_empty() {
        dest.vendor_prefix
      } else {
        *$prefix
      };
      vp.to_css(dest)?;
      vp
    }};
  }

  macro_rules! write_prefixed {
    ($prefix: ident, $val: expr) => {{
      write_prefix!($prefix);
      dest.write_str($val)
    }};
  }

  match pseudo_element {
    // CSS2 pseudo elements support a single colon syntax in addition
    // to the more correct double colon for other pseudo elements.
    // We use that here because it's supported everywhere and is shorter.
    After => dest.write_str(":after"),
    Before => dest.write_str(":before"),
    FirstLine => dest.write_str(":first-line"),
    FirstLetter => dest.write_str(":first-letter"),
    Marker => dest.write_str("::marker"),
    Selection(prefix) => write_prefixed!(prefix, "selection"),
    Cue => dest.write_str("::cue"),
    CueRegion => dest.write_str("::cue-region"),
    CueFunction { selector } => {
      dest.write_str("::cue(")?;
      serialize_selector(selector, dest, context, false)?;
      dest.write_char(')')
    }
    CueRegionFunction { selector } => {
      dest.write_str("::cue-region(")?;
      serialize_selector(selector, dest, context, false)?;
      dest.write_char(')')
    }
    Placeholder(prefix) => {
      let vp = write_prefix!(prefix);
      if vp == VendorPrefix::WebKit || vp == VendorPrefix::Ms {
        dest.write_str("input-placeholder")
      } else {
        dest.write_str("placeholder")
      }
    }
    Backdrop(prefix) => write_prefixed!(prefix, "backdrop"),
    FileSelectorButton(prefix) => {
      let vp = write_prefix!(prefix);
      if vp == VendorPrefix::WebKit {
        dest.write_str("file-upload-button")
      } else if vp == VendorPrefix::Ms {
        dest.write_str("browse")
      } else {
        dest.write_str("file-selector-button")
      }
    }
    WebKitScrollbar(s) => {
      use WebKitScrollbarPseudoElement::*;
      dest.write_str(match s {
        Scrollbar => "::-webkit-scrollbar",
        Button => "::-webkit-scrollbar-button",
        Track => "::-webkit-scrollbar-track",
        TrackPiece => "::-webkit-scrollbar-track-piece",
        Thumb => "::-webkit-scrollbar-thumb",
        Corner => "::-webkit-scrollbar-corner",
        Resizer => "::-webkit-resizer",
      })
    }
    Custom { name: val } => {
      dest.write_str("::")?;
      return dest.write_str(val);
    }
    CustomFunction { name, arguments: args } => {
      dest.write_str("::")?;
      dest.write_str(name)?;
      dest.write_char('(')?;
      args.to_css(dest, false)?;
      dest.write_char(')')
    }
  }
}

impl<'i> parcel_selectors::parser::PseudoElement<'i> for PseudoElement<'i> {
  type Impl = Selectors;

  fn accepts_state_pseudo_classes(&self) -> bool {
    // Be lenient.
    true
  }

  fn valid_after_slotted(&self) -> bool {
    // ::slotted() should support all tree-abiding pseudo-elements, see
    // https://drafts.csswg.org/css-scoping/#slotted-pseudo
    // https://drafts.csswg.org/css-pseudo-4/#treelike
    matches!(
      *self,
      PseudoElement::Before
        | PseudoElement::After
        | PseudoElement::Marker
        | PseudoElement::Placeholder(_)
        | PseudoElement::FileSelectorButton(_)
    )
  }

  fn is_webkit_scrollbar(&self) -> bool {
    matches!(*self, PseudoElement::WebKitScrollbar(..))
  }
}

impl<'i> PseudoElement<'i> {
  pub(crate) fn is_equivalent(&self, other: &PseudoElement<'i>) -> bool {
    use PseudoElement::*;
    match (self, other) {
      (Selection(_), Selection(_))
      | (Placeholder(_), Placeholder(_))
      | (Backdrop(_), Backdrop(_))
      | (FileSelectorButton(_), FileSelectorButton(_)) => true,
      (a, b) => a == b,
    }
  }

  pub(crate) fn get_prefix(&self) -> VendorPrefix {
    use PseudoElement::*;
    match self {
      Selection(p) | Placeholder(p) | Backdrop(p) | FileSelectorButton(p) => *p,
      _ => VendorPrefix::empty(),
    }
  }

  pub(crate) fn get_necessary_prefixes(&self, targets: Browsers) -> VendorPrefix {
    use crate::prefixes::Feature;
    use PseudoElement::*;
    let feature = match self {
      Selection(p) if *p == VendorPrefix::None => Feature::PseudoElementSelection,
      Placeholder(p) if *p == VendorPrefix::None => Feature::PseudoElementPlaceholder,
      Backdrop(p) if *p == VendorPrefix::None => Feature::PseudoElementBackdrop,
      FileSelectorButton(p) if *p == VendorPrefix::None => Feature::PseudoElementFileSelectorButton,
      _ => return VendorPrefix::empty(),
    };

    feature.prefixes_for(targets)
  }
}

impl<'a, 'i> ToCss for SelectorList<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: fmt::Write,
  {
    serialize_selector_list(self.0.iter(), dest, dest.context(), false)
  }
}

impl ToCss for Combinator {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: fmt::Write,
  {
    match *self {
      Combinator::Child => dest.delim('>', true),
      Combinator::Descendant => dest.write_str(" "),
      Combinator::NextSibling => dest.delim('+', true),
      Combinator::LaterSibling => dest.delim('~', true),
      Combinator::PseudoElement | Combinator::Part | Combinator::SlotAssignment => Ok(()),
    }
  }
}

// Copied from the selectors crate and modified to override to_css implementation.
impl<'a, 'i> ToCss for Selector<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: fmt::Write,
  {
    serialize_selector(self, dest, dest.context(), false)
  }
}

fn serialize_selector<'a, 'i, W>(
  selector: &Selector<'i>,
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
  mut is_relative: bool,
) -> Result<(), PrinterError>
where
  W: fmt::Write,
{
  use parcel_selectors::parser::*;
  // Compound selectors invert the order of their contents, so we need to
  // undo that during serialization.
  //
  // This two-iterator strategy involves walking over the selector twice.
  // We could do something more clever, but selector serialization probably
  // isn't hot enough to justify it, and the stringification likely
  // dominates anyway.
  //
  // NB: A parse-order iterator is a Rev<>, which doesn't expose as_slice(),
  // which we need for |split|. So we split by combinators on a match-order
  // sequence and then reverse.

  let mut combinators = selector.iter_raw_match_order().rev().filter_map(|x| x.as_combinator());
  let compound_selectors = selector.iter_raw_match_order().as_slice().split(|x| x.is_combinator()).rev();
  let supports_nesting = dest.targets.is_none() || Feature::CssNesting.is_compatible(dest.targets.unwrap());

  let mut first = true;
  let mut combinators_exhausted = false;
  for mut compound in compound_selectors {
    debug_assert!(!combinators_exhausted);

    // Skip implicit :scope in relative selectors (e.g. :has(:scope > foo) -> :has(> foo))
    if is_relative && matches!(compound.get(0), Some(Component::Scope)) {
      if let Some(combinator) = combinators.next() {
        combinator.to_css(dest)?;
      }
      compound = &compound[1..];
      is_relative = false;
    }

    // https://drafts.csswg.org/cssom/#serializing-selectors
    if compound.is_empty() {
      continue;
    }

    let has_leading_nesting = first && matches!(compound[0], Component::Nesting);
    let first_index = if has_leading_nesting { 1 } else { 0 };
    first = false;

    // 1. If there is only one simple selector in the compound selectors
    //    which is a universal selector, append the result of
    //    serializing the universal selector to s.
    //
    // Check if `!compound.empty()` first--this can happen if we have
    // something like `... > ::before`, because we store `>` and `::`
    // both as combinators internally.
    //
    // If we are in this case, after we have serialized the universal
    // selector, we skip Step 2 and continue with the algorithm.
    let (can_elide_namespace, first_non_namespace) = match compound.get(first_index) {
      Some(Component::ExplicitAnyNamespace)
      | Some(Component::ExplicitNoNamespace)
      | Some(Component::Namespace(..)) => (false, first_index + 1),
      Some(Component::DefaultNamespace(..)) => (true, first_index + 1),
      _ => (true, first_index),
    };
    let mut perform_step_2 = true;
    let next_combinator = combinators.next();
    if first_non_namespace == compound.len() - 1 {
      match (next_combinator, &compound[first_non_namespace]) {
        // We have to be careful here, because if there is a
        // pseudo element "combinator" there isn't really just
        // the one simple selector. Technically this compound
        // selector contains the pseudo element selector as well
        // -- Combinator::PseudoElement, just like
        // Combinator::SlotAssignment, don't exist in the
        // spec.
        (Some(Combinator::PseudoElement), _) | (Some(Combinator::SlotAssignment), _) => (),
        (_, &Component::ExplicitUniversalType) => {
          // Iterate over everything so we serialize the namespace
          // too.
          let mut iter = compound.iter();
          let swap_nesting = has_leading_nesting && !supports_nesting;
          if swap_nesting {
            // Swap nesting and type selector (e.g. &div -> div&).
            iter.next();
          }

          for simple in iter {
            serialize_component(simple, dest, context)?;
          }

          if swap_nesting {
            serialize_nesting(dest, context, false)?;
          }

          // Skip step 2, which is an "otherwise".
          perform_step_2 = false;
        }
        _ => (),
      }
    }

    // 2. Otherwise, for each simple selector in the compound selectors
    //    that is not a universal selector of which the namespace prefix
    //    maps to a namespace that is not the default namespace
    //    serialize the simple selector and append the result to s.
    //
    // See https://github.com/w3c/csswg-drafts/issues/1606, which is
    // proposing to change this to match up with the behavior asserted
    // in cssom/serialize-namespaced-type-selectors.html, which the
    // following code tries to match.
    if perform_step_2 {
      let mut iter = compound.iter();
      if has_leading_nesting && !supports_nesting && is_type_selector(compound.get(first_non_namespace)) {
        // Swap nesting and type selector (e.g. &div -> div&).
        // This ensures that the compiled selector is valid. e.g. (div.foo is valid, .foodiv is not).
        let nesting = iter.next().unwrap();
        let local = iter.next().unwrap();
        serialize_component(local, dest, context)?;

        // Also check the next item in case of namespaces.
        if first_non_namespace > first_index {
          let local = iter.next().unwrap();
          serialize_component(local, dest, context)?;
        }

        serialize_component(nesting, dest, context)?;
      } else if has_leading_nesting && !supports_nesting {
        // Nesting selector may serialize differently if it is leading, due to type selectors.
        iter.next();
        serialize_nesting(dest, context, true)?;
      }

      for simple in iter {
        if let Component::ExplicitUniversalType = *simple {
          // Can't have a namespace followed by a pseudo-element
          // selector followed by a universal selector in the same
          // compound selector, so we don't have to worry about the
          // real namespace being in a different `compound`.
          if can_elide_namespace {
            continue;
          }
        }
        serialize_component(simple, dest, context)?;
      }
    }

    // 3. If this is not the last part of the chain of the selector
    //    append a single SPACE (U+0020), followed by the combinator
    //    ">", "+", "~", ">>", "||", as appropriate, followed by another
    //    single SPACE (U+0020) if the combinator was not whitespace, to
    //    s.
    match next_combinator {
      Some(c) => c.to_css(dest)?,
      None => combinators_exhausted = true,
    };

    // 4. If this is the last part of the chain of the selector and
    //    there is a pseudo-element, append "::" followed by the name of
    //    the pseudo-element, to s.
    //
    // (we handle this above)
  }

  Ok(())
}

fn serialize_component<'a, 'i, W>(
  component: &Component,
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
) -> Result<(), PrinterError>
where
  W: fmt::Write,
{
  match component {
    Component::Combinator(ref c) => c.to_css(dest),
    Component::AttributeInNoNamespace {
      ref local_name,
      operator,
      ref value,
      case_sensitivity,
      ..
    } => {
      dest.write_char('[')?;
      cssparser::ToCss::to_css(local_name, dest)?;
      cssparser::ToCss::to_css(operator, dest)?;

      if dest.minify {
        // Serialize as both an identifier and a string and choose the shorter one.
        let mut id = String::new();
        serialize_identifier(&value, &mut id)?;

        let s = value.to_css_string(Default::default())?;

        if id.len() > 0 && id.len() < s.len() {
          dest.write_str(&id)?;
        } else {
          dest.write_str(&s)?;
        }
      } else {
        value.to_css(dest)?;
      }

      match case_sensitivity {
        parcel_selectors::attr::ParsedCaseSensitivity::CaseSensitive
        | parcel_selectors::attr::ParsedCaseSensitivity::AsciiCaseInsensitiveIfInHtmlElementInHtmlDocument => {}
        parcel_selectors::attr::ParsedCaseSensitivity::AsciiCaseInsensitive => dest.write_str(" i")?,
        parcel_selectors::attr::ParsedCaseSensitivity::ExplicitCaseSensitive => dest.write_str(" s")?,
      }
      dest.write_char(']')
    }
    Component::Is(ref list)
    | Component::Where(ref list)
    | Component::Negation(ref list)
    | Component::Any(_, ref list) => {
      match *component {
        Component::Where(..) => dest.write_str(":where(")?,
        Component::Is(ref selectors) => {
          // If there's only one simple selector, serialize it directly.
          if selectors.len() == 1 {
            let first = selectors.first().unwrap();
            if !has_type_selector(first) && is_simple(first) {
              serialize_selector(first, dest, context, false)?;
              return Ok(());
            }
          }

          let vp = dest.vendor_prefix;
          if vp.intersects(VendorPrefix::WebKit | VendorPrefix::Moz) {
            dest.write_char(':')?;
            vp.to_css(dest)?;
            dest.write_str("any(")?;
          } else {
            dest.write_str(":is(")?;
          }
        }
        Component::Negation(..) => return serialize_negation(list.iter(), dest, context),
        Component::Any(ref prefix, ..) => {
          dest.write_char(':')?;
          prefix.to_css(dest)?;
          dest.write_str("any(")?;
        }
        _ => unreachable!(),
      }
      serialize_selector_list(list.iter(), dest, context, false)?;
      dest.write_str(")")
    }
    Component::Has(ref list) => {
      dest.write_str(":has(")?;
      serialize_selector_list(list.iter(), dest, context, true)?;
      dest.write_str(")")
    }
    Component::NonTSPseudoClass(pseudo) => serialize_pseudo_class(pseudo, dest, context),
    Component::PseudoElement(pseudo) => serialize_pseudo_element(pseudo, dest, context),
    Component::Nesting => serialize_nesting(dest, context, false),
    Component::Class(ref class) => {
      dest.write_char('.')?;
      dest.write_ident(&class.0)
    }
    Component::ID(ref id) => {
      dest.write_char('#')?;
      dest.write_ident(&id.0)
    }
    Component::Host(selector) => {
      dest.write_str(":host")?;
      if let Some(ref selector) = *selector {
        dest.write_char('(')?;
        selector.to_css(dest)?;
        dest.write_char(')')?;
      }
      Ok(())
    }
    Component::Slotted(ref selector) => {
      dest.write_str("::slotted(")?;
      selector.to_css(dest)?;
      dest.write_char(')')
    }
    _ => {
      cssparser::ToCss::to_css(component, dest)?;
      Ok(())
    }
  }
}

fn serialize_nesting<W>(
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
  first: bool,
) -> Result<(), PrinterError>
where
  W: fmt::Write,
{
  if let Some(ctx) = context {
    // If there's only one simple selector, just serialize it directly.
    // Otherwise, use an :is() pseudo class.
    // Type selectors are only allowed at the start of a compound selector,
    // so use :is() if that is not the case.
    if ctx.selectors.0.len() == 1
      && (first || (!has_type_selector(&ctx.selectors.0[0]) && is_simple(&ctx.selectors.0[0])))
    {
      serialize_selector(ctx.selectors.0.first().unwrap(), dest, ctx.parent, false)
    } else {
      dest.write_str(":is(")?;
      serialize_selector_list(ctx.selectors.0.iter(), dest, ctx.parent, false)?;
      dest.write_char(')')
    }
  } else {
    // If there is no context, we are at the root if nesting is supported. This is equivalent to :scope.
    // Otherwise, if nesting is supported, serialize the nesting selector directly.
    let supports_nesting = dest.targets.is_none() || Feature::CssNesting.is_compatible(dest.targets.unwrap());
    if supports_nesting {
      dest.write_char('&')
    } else {
      dest.write_str(":scope")
    }
  }
}

#[inline]
fn has_type_selector(selector: &Selector) -> bool {
  let mut iter = selector.iter_raw_parse_order_from(0);
  let first = iter.next();
  if is_namespace(first) {
    is_type_selector(iter.next())
  } else {
    is_type_selector(first)
  }
}

#[inline]
fn is_simple(selector: &Selector) -> bool {
  !selector.iter_raw_match_order().any(|component| component.is_combinator())
}

#[inline]
fn is_type_selector(component: Option<&Component>) -> bool {
  matches!(
    component,
    Some(Component::LocalName(_)) | Some(Component::ExplicitUniversalType)
  )
}

#[inline]
fn is_namespace(component: Option<&Component>) -> bool {
  matches!(
    component,
    Some(Component::ExplicitAnyNamespace)
      | Some(Component::ExplicitNoNamespace)
      | Some(Component::Namespace(..))
      | Some(Component::DefaultNamespace(_))
  )
}

fn serialize_selector_list<'a, 'i: 'a, I, W>(
  iter: I,
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
  is_relative: bool,
) -> Result<(), PrinterError>
where
  I: Iterator<Item = &'a Selector<'i>>,
  W: fmt::Write,
{
  let mut first = true;
  for selector in iter {
    if !first {
      dest.delim(',', false)?;
    }
    first = false;
    serialize_selector(selector, dest, context, is_relative)?;
  }
  Ok(())
}

fn serialize_negation<'a, 'i: 'a, I, W>(
  iter: I,
  dest: &mut Printer<W>,
  context: Option<&StyleContext>,
) -> Result<(), PrinterError>
where
  I: Iterator<Item = &'a Selector<'i>>,
  W: fmt::Write,
{
  // Downlevel :not(.a, .b) -> :not(.a):not(.b) if not list is unsupported.
  let is_supported = if let Some(targets) = dest.targets {
    Feature::CssNotSelList.is_compatible(targets)
  } else {
    true
  };

  if is_supported {
    dest.write_str(":not(")?;
    serialize_selector_list(iter, dest, context, false)?;
    dest.write_char(')')?;
  } else {
    for selector in iter {
      dest.write_str(":not(")?;
      serialize_selector(selector, dest, context, false)?;
      dest.write_char(')')?;
    }
  }

  Ok(())
}

pub(crate) fn is_compatible(selectors: &SelectorList, targets: Option<Browsers>) -> bool {
  for selector in &selectors.0 {
    let iter = selector.iter();
    for component in iter {
      let feature = match component {
        Component::ID(_) | Component::Class(_) | Component::LocalName(_) => continue,

        Component::ExplicitAnyNamespace
        | Component::ExplicitNoNamespace
        | Component::DefaultNamespace(_)
        | Component::Namespace(_, _) => Feature::CssNamespaces,

        Component::ExplicitUniversalType => Feature::CssSel2,

        Component::AttributeInNoNamespaceExists { .. } => Feature::CssSel2,
        Component::AttributeInNoNamespace {
          operator,
          case_sensitivity,
          ..
        } => {
          if *case_sensitivity != ParsedCaseSensitivity::CaseSensitive {
            Feature::CssCaseInsensitive
          } else {
            match operator {
              AttrSelectorOperator::Equal | AttrSelectorOperator::Includes | AttrSelectorOperator::DashMatch => {
                Feature::CssSel2
              }
              AttrSelectorOperator::Prefix | AttrSelectorOperator::Substring | AttrSelectorOperator::Suffix => {
                Feature::CssSel3
              }
            }
          }
        }
        Component::AttributeOther(attr) => match attr.operation {
          ParsedAttrSelectorOperation::Exists => Feature::CssSel2,
          ParsedAttrSelectorOperation::WithValue {
            operator,
            case_sensitivity,
            ..
          } => {
            if case_sensitivity != ParsedCaseSensitivity::CaseSensitive {
              Feature::CssCaseInsensitive
            } else {
              match operator {
                AttrSelectorOperator::Equal | AttrSelectorOperator::Includes | AttrSelectorOperator::DashMatch => {
                  Feature::CssSel2
                }
                AttrSelectorOperator::Prefix | AttrSelectorOperator::Substring | AttrSelectorOperator::Suffix => {
                  Feature::CssSel3
                }
              }
            }
          }
        },

        Component::FirstChild => Feature::CssSel2,

        Component::Empty
        | Component::FirstOfType
        | Component::LastChild
        | Component::LastOfType
        | Component::Negation(_)
        | Component::NthChild(_, _)
        | Component::NthLastChild(_, _)
        | Component::NthCol(_, _)
        | Component::NthLastCol(_, _)
        | Component::NthLastOfType(_, _)
        | Component::NthOfType(_, _)
        | Component::OnlyChild
        | Component::OnlyOfType
        | Component::Root => Feature::CssSel3,

        Component::Is(_) | Component::Nesting => Feature::CssMatchesPseudo,
        Component::Any(..) => Feature::AnyPseudo,
        Component::Has(_) => Feature::CssHas,

        Component::Scope | Component::Host(_) | Component::Slotted(_) => Feature::Shadowdomv1,

        Component::Part(_) | Component::Where(_) => return false, // TODO: find this data in caniuse-lite

        Component::NonTSPseudoClass(pseudo) => {
          match pseudo {
            PseudoClass::Link
            | PseudoClass::Visited
            | PseudoClass::Active
            | PseudoClass::Hover
            | PseudoClass::Focus
            | PseudoClass::Lang { languages: _ } => Feature::CssSel2,

            PseudoClass::Checked | PseudoClass::Disabled | PseudoClass::Enabled | PseudoClass::Target => {
              Feature::CssSel3
            }

            PseudoClass::AnyLink(prefix) if *prefix == VendorPrefix::None => Feature::CssAnyLink,
            PseudoClass::Indeterminate => Feature::CssIndeterminatePseudo,

            PseudoClass::Fullscreen(prefix) if *prefix == VendorPrefix::None => Feature::Fullscreen,

            PseudoClass::FocusVisible => Feature::CssFocusVisible,
            PseudoClass::FocusWithin => Feature::CssFocusWithin,
            PseudoClass::Default => Feature::CssDefaultPseudo,
            PseudoClass::Dir { direction: _ } => Feature::CssDirPseudo,
            PseudoClass::Optional => Feature::CssOptionalPseudo,
            PseudoClass::PlaceholderShown(prefix) if *prefix == VendorPrefix::None => Feature::CssPlaceholderShown,

            PseudoClass::ReadOnly(prefix) | PseudoClass::ReadWrite(prefix) if *prefix == VendorPrefix::None => {
              Feature::CssReadOnlyWrite
            }

            PseudoClass::Valid | PseudoClass::Invalid | PseudoClass::Required => Feature::FormValidation,

            PseudoClass::InRange | PseudoClass::OutOfRange => Feature::CssInOutOfRange,

            PseudoClass::Autofill(prefix) if *prefix == VendorPrefix::None => Feature::CssAutofill,

            // Experimental, no browser support.
            PseudoClass::Current
            | PseudoClass::Past
            | PseudoClass::Future
            | PseudoClass::Playing
            | PseudoClass::Paused
            | PseudoClass::Seeking
            | PseudoClass::Stalled
            | PseudoClass::Buffering
            | PseudoClass::Muted
            | PseudoClass::VolumeLocked
            | PseudoClass::TargetWithin
            | PseudoClass::LocalLink
            | PseudoClass::Blank
            | PseudoClass::UserInvalid
            | PseudoClass::UserValid
            | PseudoClass::Defined => return false,

            PseudoClass::Custom { .. } | _ => return false,
          }
        }

        Component::PseudoElement(pseudo) => match pseudo {
          PseudoElement::After | PseudoElement::Before => Feature::CssGencontent,
          PseudoElement::FirstLine => Feature::CssFirstLine,
          PseudoElement::FirstLetter => Feature::CssFirstLetter,
          PseudoElement::Selection(prefix) if *prefix == VendorPrefix::None => Feature::CssSelection,
          PseudoElement::Placeholder(prefix) if *prefix == VendorPrefix::None => Feature::CssPlaceholder,
          PseudoElement::Marker => Feature::CssMarkerPseudo,
          PseudoElement::Backdrop(prefix) if *prefix == VendorPrefix::None => Feature::Dialog,
          PseudoElement::Cue => Feature::Cue,
          PseudoElement::CueFunction { selector: _ } => Feature::CueFunction,
          PseudoElement::Custom { name: _ } | _ => return false,
        },

        Component::Combinator(combinator) => match combinator {
          Combinator::Child | Combinator::NextSibling => Feature::CssSel2,
          Combinator::LaterSibling => Feature::CssSel3,
          _ => continue,
        },
      };

      if let Some(targets) = targets {
        if !feature.is_compatible(targets) {
          return false;
        }
      } else {
        return false;
      }
    }
  }

  true
}

/// Returns whether two selector lists are equivalent, i.e. the same minus any vendor prefix differences.
pub(crate) fn is_equivalent<'i>(selectors: &SelectorList<'i>, other: &SelectorList<'i>) -> bool {
  if selectors.0.len() != other.0.len() {
    return false;
  }

  for (i, a) in selectors.0.iter().enumerate() {
    let b = &other.0[i];
    if a.len() != b.len() {
      return false;
    }

    for (a, b) in a.iter().zip(b.iter()) {
      let is_equivalent = match (a, b) {
        (Component::NonTSPseudoClass(a_ps), Component::NonTSPseudoClass(b_ps)) => a_ps.is_equivalent(b_ps),
        (Component::PseudoElement(a_pe), Component::PseudoElement(b_pe)) => a_pe.is_equivalent(b_pe),
        (a, b) => a == b,
      };

      if !is_equivalent {
        return false;
      }
    }
  }

  true
}

/// Returns the vendor prefix (if any) used in the given selector list.
/// If multiple vendor prefixes are seen, this is invalid, and an empty result is returned.
pub(crate) fn get_prefix(selectors: &SelectorList) -> VendorPrefix {
  let mut prefix = VendorPrefix::empty();
  for selector in &selectors.0 {
    for component in selector.iter_raw_match_order() {
      let p = match component {
        // Return none rather than empty for these so that we call downlevel_selectors.
        Component::NonTSPseudoClass(PseudoClass::Lang { .. })
        | Component::NonTSPseudoClass(PseudoClass::Dir { .. })
        | Component::Is(..)
        | Component::Where(..)
        | Component::Has(..)
        | Component::Negation(..) => VendorPrefix::None,
        Component::Any(prefix, _) => *prefix,
        Component::NonTSPseudoClass(pc) => pc.get_prefix(),
        Component::PseudoElement(pe) => pe.get_prefix(),
        _ => VendorPrefix::empty(),
      };

      if !p.is_empty() {
        if prefix.is_empty() || prefix == p {
          prefix = p;
        } else {
          return VendorPrefix::empty();
        }
      }
    }
  }

  prefix
}

const RTL_LANGS: &[&str] = &[
  "ae", "ar", "arc", "bcc", "bqi", "ckb", "dv", "fa", "glk", "he", "ku", "mzn", "nqo", "pnb", "ps", "sd", "ug",
  "ur", "yi",
];

/// Downlevels the given selectors to be compatible with the given browser targets.
/// Returns the necessary vendor prefixes.
pub(crate) fn downlevel_selectors(selectors: &mut [Selector], targets: Browsers) -> VendorPrefix {
  let mut necessary_prefixes = VendorPrefix::empty();
  for selector in selectors {
    for component in selector.iter_mut_raw_match_order() {
      necessary_prefixes |= downlevel_component(component, targets);
    }
  }

  necessary_prefixes
}

fn downlevel_component<'i>(component: &mut Component<'i>, targets: Browsers) -> VendorPrefix {
  match component {
    Component::NonTSPseudoClass(pc) => {
      match pc {
        PseudoClass::Dir { direction: dir } => {
          if !Feature::CssDirPseudo.is_compatible(targets) {
            *component = downlevel_dir(*dir, targets);
            downlevel_component(component, targets)
          } else {
            VendorPrefix::empty()
          }
        }
        PseudoClass::Lang { languages: langs } => {
          // :lang() with multiple languages is not supported everywhere.
          // compile this to :is(:lang(a), :lang(b)) etc.
          if langs.len() > 1 && !Feature::LangList.is_compatible(targets) {
            *component = Component::Is(lang_list_to_selectors(&langs));
            downlevel_component(component, targets)
          } else {
            VendorPrefix::empty()
          }
        }
        _ => pc.get_necessary_prefixes(targets),
      }
    }
    Component::PseudoElement(pe) => pe.get_necessary_prefixes(targets),
    Component::Is(selectors) => {
      let mut necessary_prefixes = downlevel_selectors(&mut **selectors, targets);

      // Convert :is to :-webkit-any/:-moz-any if needed.
      // All selectors must be simple, no combinators are supported.
      if !Feature::CssMatchesPseudo.is_compatible(targets)
        && selectors.iter().all(|selector| !selector.has_combinator())
      {
        necessary_prefixes |= crate::prefixes::Feature::AnyPseudo.prefixes_for(targets)
      } else {
        necessary_prefixes |= VendorPrefix::empty()
      }

      necessary_prefixes
    }
    Component::Where(selectors)
    | Component::Any(_, selectors)
    | Component::Negation(selectors)
    | Component::Has(selectors) => downlevel_selectors(&mut **selectors, targets),
    _ => VendorPrefix::empty(),
  }
}

fn lang_list_to_selectors<'i>(langs: &Vec<CowArcStr<'i>>) -> Box<[Selector<'i>]> {
  langs
    .iter()
    .map(|lang| {
      Selector::from(Component::NonTSPseudoClass(PseudoClass::Lang {
        languages: vec![lang.clone()],
      }))
    })
    .collect::<Vec<Selector>>()
    .into_boxed_slice()
}

fn downlevel_dir<'i>(dir: Direction, targets: Browsers) -> Component<'i> {
  // Convert :dir to :lang. If supported, use a list of languages in a single :lang,
  // otherwise, use :is/:not, which may be further downleveled to e.g. :-webkit-any.
  let langs = RTL_LANGS.iter().map(|lang| (*lang).into()).collect();
  if Feature::LangList.is_compatible(targets) {
    let c = Component::NonTSPseudoClass(PseudoClass::Lang { languages: langs });
    if dir == Direction::Ltr {
      Component::Negation(vec![Selector::from(c)].into_boxed_slice())
    } else {
      c
    }
  } else {
    if dir == Direction::Ltr {
      Component::Negation(lang_list_to_selectors(&langs))
    } else {
      Component::Is(lang_list_to_selectors(&langs))
    }
  }
}

/// Determines whether a selector list contains only unused selectors.
/// A selector is considered unused if it contains a class or id component that exists in the set of unused symbols.
pub(crate) fn is_unused(
  selectors: &mut std::slice::Iter<Selector>,
  unused_symbols: &HashSet<String>,
  parent_is_unused: bool,
) -> bool {
  if unused_symbols.is_empty() {
    return false;
  }

  selectors.all(|selector| {
    for component in selector.iter_raw_match_order() {
      match component {
        Component::Class(name) | Component::ID(name) => {
          if unused_symbols.contains(&name.0.to_string()) {
            return true;
          }
        }
        Component::Is(is) | Component::Where(is) | Component::Any(_, is) => {
          if is_unused(&mut is.iter(), unused_symbols, parent_is_unused) {
            return true;
          }
        }
        Component::Nesting => {
          if parent_is_unused {
            return true;
          }
        }
        _ => {}
      }
    }

    false
  })
}

#[cfg(feature = "visitor")]
#[cfg_attr(docsrs, doc(cfg(feature = "visitor")))]
impl<'i, T: Visit<'i, T, V>, V: Visitor<'i, T>> Visit<'i, T, V> for SelectorList<'i> {
  const CHILD_TYPES: VisitTypes = VisitTypes::SELECTORS;

  fn visit(&mut self, visitor: &mut V) -> Result<(), V::Error> {
    if visitor.visit_types().contains(VisitTypes::SELECTORS) {
      visitor.visit_selector_list(self)
    } else {
      self.visit_children(visitor)
    }
  }

  fn visit_children(&mut self, visitor: &mut V) -> Result<(), V::Error> {
    self.0.iter_mut().try_for_each(|selector| Visit::visit(selector, visitor))
  }
}

#[cfg(feature = "visitor")]
#[cfg_attr(docsrs, doc(cfg(feature = "visitor")))]
impl<'i, T: Visit<'i, T, V>, V: Visitor<'i, T>> Visit<'i, T, V> for Selector<'i> {
  const CHILD_TYPES: VisitTypes = VisitTypes::SELECTORS;

  fn visit(&mut self, visitor: &mut V) -> Result<(), V::Error> {
    visitor.visit_selector(self)
  }

  fn visit_children(&mut self, _visitor: &mut V) -> Result<(), V::Error> {
    Ok(())
  }
}

impl<'i> ParseWithOptions<'i> for Selector<'i> {
  fn parse_with_options<'t>(
    input: &mut Parser<'i, 't>,
    options: &ParserOptions<'_, 'i>,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    Selector::parse(
      &SelectorParser {
        is_nesting_allowed: options.nesting,
        options: &options,
      },
      input,
    )
  }
}

impl<'i> ParseWithOptions<'i> for SelectorList<'i> {
  fn parse_with_options<'t>(
    input: &mut Parser<'i, 't>,
    options: &ParserOptions<'_, 'i>,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    SelectorList::parse(
      &SelectorParser {
        is_nesting_allowed: options.nesting,
        options: &options,
      },
      input,
      parcel_selectors::parser::NestingRequirement::None,
    )
  }
}
