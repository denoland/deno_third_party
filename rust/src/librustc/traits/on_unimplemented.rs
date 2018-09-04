// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use fmt_macros::{Parser, Piece, Position};

use hir::def_id::DefId;
use ty::{self, TyCtxt, GenericParamDefKind};
use util::common::ErrorReported;
use util::nodemap::FxHashMap;

use syntax::ast::{MetaItem, NestedMetaItem};
use syntax::attr;
use syntax_pos::Span;
use syntax_pos::symbol::LocalInternedString;

#[derive(Clone, Debug)]
pub struct OnUnimplementedFormatString(LocalInternedString);

#[derive(Debug)]
pub struct OnUnimplementedDirective {
    pub condition: Option<MetaItem>,
    pub subcommands: Vec<OnUnimplementedDirective>,
    pub message: Option<OnUnimplementedFormatString>,
    pub label: Option<OnUnimplementedFormatString>,
    pub note: Option<OnUnimplementedFormatString>,
}

pub struct OnUnimplementedNote {
    pub message: Option<String>,
    pub label: Option<String>,
    pub note: Option<String>,
}

impl OnUnimplementedNote {
    pub fn empty() -> Self {
        OnUnimplementedNote { message: None, label: None, note: None }
    }
}

fn parse_error(tcx: TyCtxt, span: Span,
               message: &str,
               label: &str,
               note: Option<&str>)
               -> ErrorReported
{
    let mut diag = struct_span_err!(
        tcx.sess, span, E0232, "{}", message);
    diag.span_label(span, label);
    if let Some(note) = note {
        diag.note(note);
    }
    diag.emit();
    ErrorReported
}

impl<'a, 'gcx, 'tcx> OnUnimplementedDirective {
    pub fn parse(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                 trait_def_id: DefId,
                 items: &[NestedMetaItem],
                 span: Span,
                 is_root: bool)
                 -> Result<Self, ErrorReported>
    {
        let mut errored = false;
        let mut item_iter = items.iter();

        let condition = if is_root {
            None
        } else {
            let cond = item_iter.next().ok_or_else(|| {
                parse_error(tcx, span,
                            "empty `on`-clause in `#[rustc_on_unimplemented]`",
                            "empty on-clause here",
                            None)
            })?.meta_item().ok_or_else(|| {
                parse_error(tcx, span,
                            "invalid `on`-clause in `#[rustc_on_unimplemented]`",
                            "invalid on-clause here",
                            None)
            })?;
            attr::eval_condition(cond, &tcx.sess.parse_sess, &mut |_| true);
            Some(cond.clone())
        };

        let mut message = None;
        let mut label = None;
        let mut note = None;
        let mut subcommands = vec![];
        for item in item_iter {
            if item.check_name("message") && message.is_none() {
                if let Some(message_) = item.value_str() {
                    message = Some(OnUnimplementedFormatString::try_parse(
                        tcx, trait_def_id, message_.as_str(), span)?);
                    continue;
                }
            } else if item.check_name("label") && label.is_none() {
                if let Some(label_) = item.value_str() {
                    label = Some(OnUnimplementedFormatString::try_parse(
                        tcx, trait_def_id, label_.as_str(), span)?);
                    continue;
                }
            } else if item.check_name("note") && note.is_none() {
                if let Some(note_) = item.value_str() {
                    note = Some(OnUnimplementedFormatString::try_parse(
                        tcx, trait_def_id, note_.as_str(), span)?);
                    continue;
                }
            } else if item.check_name("on") && is_root &&
                message.is_none() && label.is_none() && note.is_none()
            {
                if let Some(items) = item.meta_item_list() {
                    if let Ok(subcommand) =
                        Self::parse(tcx, trait_def_id, &items, item.span, false)
                    {
                        subcommands.push(subcommand);
                    } else {
                        errored = true;
                    }
                    continue
                }
            }

            // nothing found
            parse_error(tcx, item.span,
                        "this attribute must have a valid value",
                        "expected value here",
                        Some(r#"eg `#[rustc_on_unimplemented = "foo"]`"#));
        }

        if errored {
            Err(ErrorReported)
        } else {
            Ok(OnUnimplementedDirective { condition, message, label, subcommands, note })
        }
    }


    pub fn of_item(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                   trait_def_id: DefId,
                   impl_def_id: DefId)
                   -> Result<Option<Self>, ErrorReported>
    {
        let attrs = tcx.get_attrs(impl_def_id);

        let attr = if let Some(item) = attr::find_by_name(&attrs, "rustc_on_unimplemented") {
            item
        } else {
            return Ok(None);
        };

        let result = if let Some(items) = attr.meta_item_list() {
            Self::parse(tcx, trait_def_id, &items, attr.span, true).map(Some)
        } else if let Some(value) = attr.value_str() {
            Ok(Some(OnUnimplementedDirective {
                condition: None,
                message: None,
                subcommands: vec![],
                label: Some(OnUnimplementedFormatString::try_parse(
                    tcx, trait_def_id, value.as_str(), attr.span)?),
                note: None,
            }))
        } else {
            return Err(parse_error(tcx, attr.span,
                                   "`#[rustc_on_unimplemented]` requires a value",
                                   "value required here",
                                   Some(r#"eg `#[rustc_on_unimplemented = "foo"]`"#)));
        };
        debug!("of_item({:?}/{:?}) = {:?}", trait_def_id, impl_def_id, result);
        result
    }

    pub fn evaluate(&self,
                    tcx: TyCtxt<'a, 'gcx, 'tcx>,
                    trait_ref: ty::TraitRef<'tcx>,
                    options: &[(String, Option<String>)])
                    -> OnUnimplementedNote
    {
        let mut message = None;
        let mut label = None;
        let mut note = None;
        info!("evaluate({:?}, trait_ref={:?}, options={:?})", self, trait_ref, options);

        for command in self.subcommands.iter().chain(Some(self)).rev() {
            if let Some(ref condition) = command.condition {
                if !attr::eval_condition(condition, &tcx.sess.parse_sess, &mut |c| {
                    options.contains(&(c.name().as_str().to_string(),
                                      match c.value_str().map(|s| s.as_str().to_string()) {
                                          Some(s) => Some(s),
                                          None => None
                                      }))
                }) {
                    debug!("evaluate: skipping {:?} due to condition", command);
                    continue
                }
            }
            debug!("evaluate: {:?} succeeded", command);
            if let Some(ref message_) = command.message {
                message = Some(message_.clone());
            }

            if let Some(ref label_) = command.label {
                label = Some(label_.clone());
            }

            if let Some(ref note_) = command.note {
                note = Some(note_.clone());
            }
        }

        OnUnimplementedNote {
            label: label.map(|l| l.format(tcx, trait_ref)),
            message: message.map(|m| m.format(tcx, trait_ref)),
            note: note.map(|n| n.format(tcx, trait_ref)),
        }
    }
}

impl<'a, 'gcx, 'tcx> OnUnimplementedFormatString {
    pub fn try_parse(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                     trait_def_id: DefId,
                     from: LocalInternedString,
                     err_sp: Span)
                     -> Result<Self, ErrorReported>
    {
        let result = OnUnimplementedFormatString(from);
        result.verify(tcx, trait_def_id, err_sp)?;
        Ok(result)
    }

    fn verify(&self,
              tcx: TyCtxt<'a, 'gcx, 'tcx>,
              trait_def_id: DefId,
              span: Span)
              -> Result<(), ErrorReported>
    {
        let name = tcx.item_name(trait_def_id);
        let generics = tcx.generics_of(trait_def_id);
        let parser = Parser::new(&self.0);
        let mut result = Ok(());
        for token in parser {
            match token {
                Piece::String(_) => (), // Normal string, no need to check it
                Piece::NextArgument(a) => match a.position {
                    // `{Self}` is allowed
                    Position::ArgumentNamed(s) if s == "Self" => (),
                    // `{ThisTraitsName}` is allowed
                    Position::ArgumentNamed(s) if s == name => (),
                    // So is `{A}` if A is a type parameter
                    Position::ArgumentNamed(s) => match generics.params.iter().find(|param| {
                        param.name == s
                    }) {
                        Some(_) => (),
                        None => {
                            span_err!(tcx.sess, span, E0230,
                                      "there is no parameter \
                                       {} on trait {}",
                                      s, name);
                            result = Err(ErrorReported);
                        }
                    },
                    // `{:1}` and `{}` are not to be used
                    Position::ArgumentIs(_) | Position::ArgumentImplicitlyIs(_) => {
                        span_err!(tcx.sess, span, E0231,
                                  "only named substitution \
                                   parameters are allowed");
                        result = Err(ErrorReported);
                    }
                }
            }
        }

        result
    }

    pub fn format(&self,
                  tcx: TyCtxt<'a, 'gcx, 'tcx>,
                  trait_ref: ty::TraitRef<'tcx>)
                  -> String
    {
        let name = tcx.item_name(trait_ref.def_id);
        let trait_str = tcx.item_path_str(trait_ref.def_id);
        let generics = tcx.generics_of(trait_ref.def_id);
        let generic_map = generics.params.iter().filter_map(|param| {
            let value = match param.kind {
                GenericParamDefKind::Type {..} => {
                    trait_ref.substs[param.index as usize].to_string()
                },
                GenericParamDefKind::Lifetime => return None
            };
            let name = param.name.to_string();
            Some((name, value))
        }).collect::<FxHashMap<String, String>>();

        let parser = Parser::new(&self.0);
        parser.map(|p| {
            match p {
                Piece::String(s) => s,
                Piece::NextArgument(a) => match a.position {
                    Position::ArgumentNamed(s) => match generic_map.get(s) {
                        Some(val) => val,
                        None if s == name => {
                            &trait_str
                        }
                        None => {
                            bug!("broken on_unimplemented {:?} for {:?}: \
                                  no argument matching {:?}",
                                 self.0, trait_ref, s)
                        }
                    },
                    _ => {
                        bug!("broken on_unimplemented {:?} - bad \
                              format arg", self.0)
                    }
                }
            }
        }).collect()
    }
}
