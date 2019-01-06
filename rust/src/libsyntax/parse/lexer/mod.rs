use ast::{self, Ident};
use syntax_pos::{self, BytePos, CharPos, Pos, Span, NO_EXPANSION};
use source_map::{SourceMap, FilePathMapping};
use errors::{Applicability, FatalError, Diagnostic, DiagnosticBuilder};
use parse::{token, ParseSess};
use symbol::{Symbol, keywords};
use core::unicode::property::Pattern_White_Space;

use std::borrow::Cow;
use std::char;
use std::iter;
use std::mem::replace;
use rustc_data_structures::sync::Lrc;

pub mod comments;
mod tokentrees;
mod unicode_chars;

#[derive(Clone, Debug)]
pub struct TokenAndSpan {
    pub tok: token::Token,
    pub sp: Span,
}

impl Default for TokenAndSpan {
    fn default() -> Self {
        TokenAndSpan {
            tok: token::Whitespace,
            sp: syntax_pos::DUMMY_SP,
        }
    }
}

pub struct StringReader<'a> {
    pub sess: &'a ParseSess,
    /// The absolute offset within the source_map of the next character to read
    pub next_pos: BytePos,
    /// The absolute offset within the source_map of the current character
    pub pos: BytePos,
    /// The current character (which has been read from self.pos)
    pub ch: Option<char>,
    pub source_file: Lrc<syntax_pos::SourceFile>,
    /// Stop reading src at this index.
    pub end_src_index: usize,
    // cached:
    peek_tok: token::Token,
    peek_span: Span,
    peek_span_src_raw: Span,
    fatal_errs: Vec<DiagnosticBuilder<'a>>,
    // cache a direct reference to the source text, so that we don't have to
    // retrieve it via `self.source_file.src.as_ref().unwrap()` all the time.
    src: Lrc<String>,
    token: token::Token,
    span: Span,
    /// The raw source span which *does not* take `override_span` into account
    span_src_raw: Span,
    /// Stack of open delimiters and their spans. Used for error message.
    open_braces: Vec<(token::DelimToken, Span)>,
    /// The type and spans for all braces
    ///
    /// Used only for error recovery when arriving to EOF with mismatched braces.
    matching_delim_spans: Vec<(token::DelimToken, Span, Span)>,
    crate override_span: Option<Span>,
    last_unclosed_found_span: Option<Span>,
}

impl<'a> StringReader<'a> {
    fn mk_sp(&self, lo: BytePos, hi: BytePos) -> Span {
        self.mk_sp_and_raw(lo, hi).0
    }

    fn mk_sp_and_raw(&self, lo: BytePos, hi: BytePos) -> (Span, Span) {
        let raw = Span::new(lo, hi, NO_EXPANSION);
        let real = self.override_span.unwrap_or(raw);

        (real, raw)
    }

    fn mk_ident(&self, string: &str) -> Ident {
        let mut ident = Ident::from_str(string);
        if let Some(span) = self.override_span {
            ident.span = span;
        }

        ident
    }

    fn unwrap_or_abort(&mut self, res: Result<TokenAndSpan, ()>) -> TokenAndSpan {
        match res {
            Ok(tok) => tok,
            Err(_) => {
                self.emit_fatal_errors();
                FatalError.raise();
            }
        }
    }

    fn next_token(&mut self) -> TokenAndSpan where Self: Sized {
        let res = self.try_next_token();
        self.unwrap_or_abort(res)
    }

    /// Return the next token. EFFECT: advances the string_reader.
    pub fn try_next_token(&mut self) -> Result<TokenAndSpan, ()> {
        assert!(self.fatal_errs.is_empty());
        let ret_val = TokenAndSpan {
            tok: replace(&mut self.peek_tok, token::Whitespace),
            sp: self.peek_span,
        };
        self.advance_token()?;
        self.span_src_raw = self.peek_span_src_raw;

        Ok(ret_val)
    }

    fn try_real_token(&mut self) -> Result<TokenAndSpan, ()> {
        let mut t = self.try_next_token()?;
        loop {
            match t.tok {
                token::Whitespace | token::Comment | token::Shebang(_) => {
                    t = self.try_next_token()?;
                }
                _ => break,
            }
        }

        self.token = t.tok.clone();
        self.span = t.sp;

        Ok(t)
    }

    pub fn real_token(&mut self) -> TokenAndSpan {
        let res = self.try_real_token();
        self.unwrap_or_abort(res)
    }

    #[inline]
    fn is_eof(&self) -> bool {
        self.ch.is_none()
    }

    fn fail_unterminated_raw_string(&self, pos: BytePos, hash_count: u16) {
        let mut err = self.struct_span_fatal(pos, pos, "unterminated raw string");
        err.span_label(self.mk_sp(pos, pos), "unterminated raw string");

        if hash_count > 0 {
            err.note(&format!("this raw string should be terminated with `\"{}`",
                              "#".repeat(hash_count as usize)));
        }

        err.emit();
        FatalError.raise();
    }

    fn fatal(&self, m: &str) -> FatalError {
        self.fatal_span(self.peek_span, m)
    }

    pub fn emit_fatal_errors(&mut self) {
        for err in &mut self.fatal_errs {
            err.emit();
        }

        self.fatal_errs.clear();
    }

    pub fn buffer_fatal_errors(&mut self) -> Vec<Diagnostic> {
        let mut buffer = Vec::new();

        for err in self.fatal_errs.drain(..) {
            err.buffer(&mut buffer);
        }

        buffer
    }

    pub fn peek(&self) -> TokenAndSpan {
        // FIXME(pcwalton): Bad copy!
        TokenAndSpan {
            tok: self.peek_tok.clone(),
            sp: self.peek_span,
        }
    }

    /// For comments.rs, which hackily pokes into next_pos and ch
    fn new_raw(sess: &'a ParseSess,
               source_file: Lrc<syntax_pos::SourceFile>,
               override_span: Option<Span>) -> Self {
        let mut sr = StringReader::new_raw_internal(sess, source_file, override_span);
        sr.bump();

        sr
    }

    fn new_raw_internal(sess: &'a ParseSess, source_file: Lrc<syntax_pos::SourceFile>,
        override_span: Option<Span>) -> Self
    {
        if source_file.src.is_none() {
            sess.span_diagnostic.bug(&format!("Cannot lex source_file without source: {}",
                                              source_file.name));
        }

        let src = (*source_file.src.as_ref().unwrap()).clone();

        StringReader {
            sess,
            next_pos: source_file.start_pos,
            pos: source_file.start_pos,
            ch: Some('\n'),
            source_file,
            end_src_index: src.len(),
            // dummy values; not read
            peek_tok: token::Eof,
            peek_span: syntax_pos::DUMMY_SP,
            peek_span_src_raw: syntax_pos::DUMMY_SP,
            src,
            fatal_errs: Vec::new(),
            token: token::Eof,
            span: syntax_pos::DUMMY_SP,
            span_src_raw: syntax_pos::DUMMY_SP,
            open_braces: Vec::new(),
            matching_delim_spans: Vec::new(),
            override_span,
            last_unclosed_found_span: None,
        }
    }

    pub fn new(sess: &'a ParseSess,
               source_file: Lrc<syntax_pos::SourceFile>,
               override_span: Option<Span>) -> Self {
        let mut sr = StringReader::new_raw(sess, source_file, override_span);
        if sr.advance_token().is_err() {
            sr.emit_fatal_errors();
            FatalError.raise();
        }

        sr
    }

    pub fn new_without_err(sess: &'a ParseSess,
                           source_file: Lrc<syntax_pos::SourceFile>,
                           override_span: Option<Span>,
                           prepend_error_text: &str) -> Result<Self, ()> {
        let mut sr = StringReader::new_raw(sess, source_file, override_span);
        if sr.advance_token().is_err() {
            eprintln!("{}", prepend_error_text);
            sr.emit_fatal_errors();
            return Err(());
        }
        Ok(sr)
    }

    pub fn new_or_buffered_errs(sess: &'a ParseSess,
                                source_file: Lrc<syntax_pos::SourceFile>,
                                override_span: Option<Span>) -> Result<Self, Vec<Diagnostic>> {
        let mut sr = StringReader::new_raw(sess, source_file, override_span);
        if sr.advance_token().is_err() {
            Err(sr.buffer_fatal_errors())
        } else {
            Ok(sr)
        }
    }

    pub fn retokenize(sess: &'a ParseSess, mut span: Span) -> Self {
        let begin = sess.source_map().lookup_byte_offset(span.lo());
        let end = sess.source_map().lookup_byte_offset(span.hi());

        // Make the range zero-length if the span is invalid.
        if span.lo() > span.hi() || begin.sf.start_pos != end.sf.start_pos {
            span = span.shrink_to_lo();
        }

        let mut sr = StringReader::new_raw_internal(sess, begin.sf, None);

        // Seek the lexer to the right byte range.
        sr.next_pos = span.lo();
        sr.end_src_index = sr.src_index(span.hi());

        sr.bump();

        if sr.advance_token().is_err() {
            sr.emit_fatal_errors();
            FatalError.raise();
        }

        sr
    }

    #[inline]
    fn ch_is(&self, c: char) -> bool {
        self.ch == Some(c)
    }

    /// Report a fatal lexical error with a given span.
    fn fatal_span(&self, sp: Span, m: &str) -> FatalError {
        self.sess.span_diagnostic.span_fatal(sp, m)
    }

    /// Report a lexical error with a given span.
    fn err_span(&self, sp: Span, m: &str) {
        self.sess.span_diagnostic.span_err(sp, m)
    }


    /// Report a fatal error spanning [`from_pos`, `to_pos`).
    fn fatal_span_(&self, from_pos: BytePos, to_pos: BytePos, m: &str) -> FatalError {
        self.fatal_span(self.mk_sp(from_pos, to_pos), m)
    }

    /// Report a lexical error spanning [`from_pos`, `to_pos`).
    fn err_span_(&self, from_pos: BytePos, to_pos: BytePos, m: &str) {
        self.err_span(self.mk_sp(from_pos, to_pos), m)
    }

    /// Pushes a character to a message string for error reporting
    fn push_escaped_char_for_msg(m: &mut String, c: char) {
        match c {
            '\u{20}'..='\u{7e}' => {
                // Don't escape \, ' or " for user-facing messages
                m.push(c);
            }
            _ => {
                m.extend(c.escape_default());
            }
        }
    }

    /// Report a lexical error spanning [`from_pos`, `to_pos`), appending an
    /// escaped character to the error message
    fn fatal_span_char(&self, from_pos: BytePos, to_pos: BytePos, m: &str, c: char) -> FatalError {
        let mut m = m.to_string();
        m.push_str(": ");
        Self::push_escaped_char_for_msg(&mut m, c);

        self.fatal_span_(from_pos, to_pos, &m[..])
    }

    fn struct_span_fatal(&self, from_pos: BytePos, to_pos: BytePos, m: &str)
        -> DiagnosticBuilder<'a>
    {
        self.sess.span_diagnostic.struct_span_fatal(self.mk_sp(from_pos, to_pos), m)
    }

    fn struct_fatal_span_char(&self, from_pos: BytePos, to_pos: BytePos, m: &str, c: char)
        -> DiagnosticBuilder<'a>
    {
        let mut m = m.to_string();
        m.push_str(": ");
        Self::push_escaped_char_for_msg(&mut m, c);

        self.sess.span_diagnostic.struct_span_fatal(self.mk_sp(from_pos, to_pos), &m[..])
    }

    /// Report a lexical error spanning [`from_pos`, `to_pos`), appending an
    /// escaped character to the error message
    fn err_span_char(&self, from_pos: BytePos, to_pos: BytePos, m: &str, c: char) {
        let mut m = m.to_string();
        m.push_str(": ");
        Self::push_escaped_char_for_msg(&mut m, c);
        self.err_span_(from_pos, to_pos, &m[..]);
    }

    fn struct_err_span_char(&self, from_pos: BytePos, to_pos: BytePos, m: &str, c: char)
        -> DiagnosticBuilder<'a>
    {
        let mut m = m.to_string();
        m.push_str(": ");
        Self::push_escaped_char_for_msg(&mut m, c);

        self.sess.span_diagnostic.struct_span_err(self.mk_sp(from_pos, to_pos), &m[..])
    }

    /// Report a lexical error spanning [`from_pos`, `to_pos`), appending the
    /// offending string to the error message
    fn fatal_span_verbose(&self, from_pos: BytePos, to_pos: BytePos, mut m: String) -> FatalError {
        m.push_str(": ");
        m.push_str(&self.src[self.src_index(from_pos)..self.src_index(to_pos)]);

        self.fatal_span_(from_pos, to_pos, &m[..])
    }

    /// Advance peek_tok and peek_span to refer to the next token, and
    /// possibly update the interner.
    fn advance_token(&mut self) -> Result<(), ()> {
        match self.scan_whitespace_or_comment() {
            Some(comment) => {
                self.peek_span_src_raw = comment.sp;
                self.peek_span = comment.sp;
                self.peek_tok = comment.tok;
            }
            None => {
                if self.is_eof() {
                    self.peek_tok = token::Eof;
                    let (real, raw) = self.mk_sp_and_raw(
                        self.source_file.end_pos,
                        self.source_file.end_pos,
                    );
                    self.peek_span = real;
                    self.peek_span_src_raw = raw;
                } else {
                    let start_bytepos = self.pos;
                    self.peek_tok = self.next_token_inner()?;
                    let (real, raw) = self.mk_sp_and_raw(start_bytepos, self.pos);
                    self.peek_span = real;
                    self.peek_span_src_raw = raw;
                };
            }
        }

        Ok(())
    }

    #[inline]
    fn src_index(&self, pos: BytePos) -> usize {
        (pos - self.source_file.start_pos).to_usize()
    }

    /// Calls `f` with a string slice of the source text spanning from `start`
    /// up to but excluding `self.pos`, meaning the slice does not include
    /// the character `self.ch`.
    fn with_str_from<T, F>(&self, start: BytePos, f: F) -> T
        where F: FnOnce(&str) -> T
    {
        self.with_str_from_to(start, self.pos, f)
    }

    /// Create a Name from a given offset to the current offset, each
    /// adjusted 1 towards each other (assumes that on either side there is a
    /// single-byte delimiter).
    fn name_from(&self, start: BytePos) -> ast::Name {
        debug!("taking an ident from {:?} to {:?}", start, self.pos);
        self.with_str_from(start, Symbol::intern)
    }

    /// As name_from, with an explicit endpoint.
    fn name_from_to(&self, start: BytePos, end: BytePos) -> ast::Name {
        debug!("taking an ident from {:?} to {:?}", start, end);
        self.with_str_from_to(start, end, Symbol::intern)
    }

    /// Calls `f` with a string slice of the source text spanning from `start`
    /// up to but excluding `end`.
    fn with_str_from_to<T, F>(&self, start: BytePos, end: BytePos, f: F) -> T
        where F: FnOnce(&str) -> T
    {
        f(&self.src[self.src_index(start)..self.src_index(end)])
    }

    /// Converts CRLF to LF in the given string, raising an error on bare CR.
    fn translate_crlf<'b>(&self, start: BytePos, s: &'b str, errmsg: &'b str) -> Cow<'b, str> {
        let mut chars = s.char_indices().peekable();
        while let Some((i, ch)) = chars.next() {
            if ch == '\r' {
                if let Some((lf_idx, '\n')) = chars.peek() {
                    return translate_crlf_(self, start, s, *lf_idx, chars, errmsg).into();
                }
                let pos = start + BytePos(i as u32);
                let end_pos = start + BytePos((i + ch.len_utf8()) as u32);
                self.err_span_(pos, end_pos, errmsg);
            }
        }
        return s.into();

        fn translate_crlf_(rdr: &StringReader,
                           start: BytePos,
                           s: &str,
                           mut j: usize,
                           mut chars: iter::Peekable<impl Iterator<Item = (usize, char)>>,
                           errmsg: &str)
                           -> String {
            let mut buf = String::with_capacity(s.len());
            // Skip first CR
            buf.push_str(&s[.. j - 1]);
            while let Some((i, ch)) = chars.next() {
                if ch == '\r' {
                    if j < i {
                        buf.push_str(&s[j..i]);
                    }
                    let next = i + ch.len_utf8();
                    j = next;
                    if chars.peek().map(|(_, ch)| *ch) != Some('\n') {
                        let pos = start + BytePos(i as u32);
                        let end_pos = start + BytePos(next as u32);
                        rdr.err_span_(pos, end_pos, errmsg);
                    }
                }
            }
            if j < s.len() {
                buf.push_str(&s[j..]);
            }
            buf
        }
    }

    /// Advance the StringReader by one character.
    crate fn bump(&mut self) {
        let next_src_index = self.src_index(self.next_pos);
        if next_src_index < self.end_src_index {
            let next_ch = char_at(&self.src, next_src_index);
            let next_ch_len = next_ch.len_utf8();

            self.ch = Some(next_ch);
            self.pos = self.next_pos;
            self.next_pos = self.next_pos + Pos::from_usize(next_ch_len);
        } else {
            self.ch = None;
            self.pos = self.next_pos;
        }
    }

    fn nextch(&self) -> Option<char> {
        let next_src_index = self.src_index(self.next_pos);
        if next_src_index < self.end_src_index {
            Some(char_at(&self.src, next_src_index))
        } else {
            None
        }
    }

    #[inline]
    fn nextch_is(&self, c: char) -> bool {
        self.nextch() == Some(c)
    }

    fn nextnextch(&self) -> Option<char> {
        let next_src_index = self.src_index(self.next_pos);
        if next_src_index < self.end_src_index {
            let next_next_src_index =
                next_src_index + char_at(&self.src, next_src_index).len_utf8();
            if next_next_src_index < self.end_src_index {
                return Some(char_at(&self.src, next_next_src_index));
            }
        }
        None
    }

    #[inline]
    fn nextnextch_is(&self, c: char) -> bool {
        self.nextnextch() == Some(c)
    }

    /// Eats <XID_start><XID_continue>*, if possible.
    fn scan_optional_raw_name(&mut self) -> Option<ast::Name> {
        if !ident_start(self.ch) {
            return None;
        }

        let start = self.pos;
        self.bump();

        while ident_continue(self.ch) {
            self.bump();
        }

        self.with_str_from(start, |string| {
            if string == "_" {
                self.sess.span_diagnostic
                    .struct_span_warn(self.mk_sp(start, self.pos),
                                      "underscore literal suffix is not allowed")
                    .warn("this was previously accepted by the compiler but is \
                          being phased out; it will become a hard error in \
                          a future release!")
                    .note("for more information, see issue #42326 \
                          <https://github.com/rust-lang/rust/issues/42326>")
                    .emit();
                None
            } else {
                Some(Symbol::intern(string))
            }
        })
    }

    /// PRECONDITION: self.ch is not whitespace
    /// Eats any kind of comment.
    fn scan_comment(&mut self) -> Option<TokenAndSpan> {
        if let Some(c) = self.ch {
            if c.is_whitespace() {
                let msg = "called consume_any_line_comment, but there was whitespace";
                self.sess.span_diagnostic.span_err(self.mk_sp(self.pos, self.pos), msg);
            }
        }

        if self.ch_is('/') {
            match self.nextch() {
                Some('/') => {
                    self.bump();
                    self.bump();

                    // line comments starting with "///" or "//!" are doc-comments
                    let doc_comment = (self.ch_is('/') && !self.nextch_is('/')) || self.ch_is('!');
                    let start_bpos = self.pos - BytePos(2);

                    while !self.is_eof() {
                        match self.ch.unwrap() {
                            '\n' => break,
                            '\r' => {
                                if self.nextch_is('\n') {
                                    // CRLF
                                    break;
                                } else if doc_comment {
                                    self.err_span_(self.pos,
                                                   self.next_pos,
                                                   "bare CR not allowed in doc-comment");
                                }
                            }
                            _ => (),
                        }
                        self.bump();
                    }

                    if doc_comment {
                        self.with_str_from(start_bpos, |string| {
                            // comments with only more "/"s are not doc comments
                            let tok = if is_doc_comment(string) {
                                token::DocComment(Symbol::intern(string))
                            } else {
                                token::Comment
                            };

                            Some(TokenAndSpan {
                                tok,
                                sp: self.mk_sp(start_bpos, self.pos),
                            })
                        })
                    } else {
                        Some(TokenAndSpan {
                            tok: token::Comment,
                            sp: self.mk_sp(start_bpos, self.pos),
                        })
                    }
                }
                Some('*') => {
                    self.bump();
                    self.bump();
                    self.scan_block_comment()
                }
                _ => None,
            }
        } else if self.ch_is('#') {
            if self.nextch_is('!') {

                // Parse an inner attribute.
                if self.nextnextch_is('[') {
                    return None;
                }

                // I guess this is the only way to figure out if
                // we're at the beginning of the file...
                let smap = SourceMap::new(FilePathMapping::empty());
                smap.files.borrow_mut().source_files.push(self.source_file.clone());
                let loc = smap.lookup_char_pos_adj(self.pos);
                debug!("Skipping a shebang");
                if loc.line == 1 && loc.col == CharPos(0) {
                    // FIXME: Add shebang "token", return it
                    let start = self.pos;
                    while !self.ch_is('\n') && !self.is_eof() {
                        self.bump();
                    }
                    return Some(TokenAndSpan {
                        tok: token::Shebang(self.name_from(start)),
                        sp: self.mk_sp(start, self.pos),
                    });
                }
            }
            None
        } else {
            None
        }
    }

    /// If there is whitespace, shebang, or a comment, scan it. Otherwise,
    /// return None.
    fn scan_whitespace_or_comment(&mut self) -> Option<TokenAndSpan> {
        match self.ch.unwrap_or('\0') {
            // # to handle shebang at start of file -- this is the entry point
            // for skipping over all "junk"
            '/' | '#' => {
                let c = self.scan_comment();
                debug!("scanning a comment {:?}", c);
                c
            },
            c if is_pattern_whitespace(Some(c)) => {
                let start_bpos = self.pos;
                while is_pattern_whitespace(self.ch) {
                    self.bump();
                }
                let c = Some(TokenAndSpan {
                    tok: token::Whitespace,
                    sp: self.mk_sp(start_bpos, self.pos),
                });
                debug!("scanning whitespace: {:?}", c);
                c
            }
            _ => None,
        }
    }

    /// Might return a sugared-doc-attr
    fn scan_block_comment(&mut self) -> Option<TokenAndSpan> {
        // block comments starting with "/**" or "/*!" are doc-comments
        let is_doc_comment = self.ch_is('*') || self.ch_is('!');
        let start_bpos = self.pos - BytePos(2);

        let mut level: isize = 1;
        let mut has_cr = false;
        while level > 0 {
            if self.is_eof() {
                let msg = if is_doc_comment {
                    "unterminated block doc-comment"
                } else {
                    "unterminated block comment"
                };
                let last_bpos = self.pos;
                self.fatal_span_(start_bpos, last_bpos, msg).raise();
            }
            let n = self.ch.unwrap();
            match n {
                '/' if self.nextch_is('*') => {
                    level += 1;
                    self.bump();
                }
                '*' if self.nextch_is('/') => {
                    level -= 1;
                    self.bump();
                }
                '\r' => {
                    has_cr = true;
                }
                _ => (),
            }
            self.bump();
        }

        self.with_str_from(start_bpos, |string| {
            // but comments with only "*"s between two "/"s are not
            let tok = if is_block_doc_comment(string) {
                let string = if has_cr {
                    self.translate_crlf(start_bpos,
                                        string,
                                        "bare CR not allowed in block doc-comment")
                } else {
                    string.into()
                };
                token::DocComment(Symbol::intern(&string[..]))
            } else {
                token::Comment
            };

            Some(TokenAndSpan {
                tok,
                sp: self.mk_sp(start_bpos, self.pos),
            })
        })
    }

    /// Scan through any digits (base `scan_radix`) or underscores,
    /// and return how many digits there were.
    ///
    /// `real_radix` represents the true radix of the number we're
    /// interested in, and errors will be emitted for any digits
    /// between `real_radix` and `scan_radix`.
    fn scan_digits(&mut self, real_radix: u32, scan_radix: u32) -> usize {
        assert!(real_radix <= scan_radix);
        let mut len = 0;

        loop {
            let c = self.ch;
            if c == Some('_') {
                debug!("skipping a _");
                self.bump();
                continue;
            }
            match c.and_then(|cc| cc.to_digit(scan_radix)) {
                Some(_) => {
                    debug!("{:?} in scan_digits", c);
                    // check that the hypothetical digit is actually
                    // in range for the true radix
                    if c.unwrap().to_digit(real_radix).is_none() {
                        self.err_span_(self.pos,
                                       self.next_pos,
                                       &format!("invalid digit for a base {} literal", real_radix));
                    }
                    len += 1;
                    self.bump();
                }
                _ => return len,
            }
        }
    }

    /// Lex a LIT_INTEGER or a LIT_FLOAT
    fn scan_number(&mut self, c: char) -> token::Lit {
        let mut base = 10;
        let start_bpos = self.pos;
        self.bump();

        let num_digits = if c == '0' {
            match self.ch.unwrap_or('\0') {
                'b' => {
                    self.bump();
                    base = 2;
                    self.scan_digits(2, 10)
                }
                'o' => {
                    self.bump();
                    base = 8;
                    self.scan_digits(8, 10)
                }
                'x' => {
                    self.bump();
                    base = 16;
                    self.scan_digits(16, 16)
                }
                '0'..='9' | '_' | '.' | 'e' | 'E' => {
                    self.scan_digits(10, 10) + 1
                }
                _ => {
                    // just a 0
                    return token::Integer(self.name_from(start_bpos));
                }
            }
        } else if c.is_digit(10) {
            self.scan_digits(10, 10) + 1
        } else {
            0
        };

        if num_digits == 0 {
            self.err_span_(start_bpos, self.pos, "no valid digits found for number");

            return token::Integer(Symbol::intern("0"));
        }

        // might be a float, but don't be greedy if this is actually an
        // integer literal followed by field/method access or a range pattern
        // (`0..2` and `12.foo()`)
        if self.ch_is('.') && !self.nextch_is('.') &&
           !ident_start(self.nextch()) {
            // might have stuff after the ., and if it does, it needs to start
            // with a number
            self.bump();
            if self.ch.unwrap_or('\0').is_digit(10) {
                self.scan_digits(10, 10);
                self.scan_float_exponent();
            }
            let pos = self.pos;
            self.check_float_base(start_bpos, pos, base);

            token::Float(self.name_from(start_bpos))
        } else {
            // it might be a float if it has an exponent
            if self.ch_is('e') || self.ch_is('E') {
                self.scan_float_exponent();
                let pos = self.pos;
                self.check_float_base(start_bpos, pos, base);
                return token::Float(self.name_from(start_bpos));
            }
            // but we certainly have an integer!
            token::Integer(self.name_from(start_bpos))
        }
    }

    /// Scan over `n_digits` hex digits, stopping at `delim`, reporting an
    /// error if too many or too few digits are encountered.
    fn scan_hex_digits(&mut self, n_digits: usize, delim: char, below_0x7f_only: bool) -> bool {
        debug!("scanning {} digits until {:?}", n_digits, delim);
        let start_bpos = self.pos;
        let mut accum_int = 0;

        let mut valid = true;
        for _ in 0..n_digits {
            if self.is_eof() {
                let last_bpos = self.pos;
                self.fatal_span_(start_bpos,
                                 last_bpos,
                                 "unterminated numeric character escape").raise();
            }
            if self.ch_is(delim) {
                let last_bpos = self.pos;
                self.err_span_(start_bpos,
                               last_bpos,
                               "numeric character escape is too short");
                valid = false;
                break;
            }
            let c = self.ch.unwrap_or('\x00');
            accum_int *= 16;
            accum_int += c.to_digit(16).unwrap_or_else(|| {
                self.err_span_char(self.pos,
                                   self.next_pos,
                                   "invalid character in numeric character escape",
                                   c);

                valid = false;
                0
            });
            self.bump();
        }

        if below_0x7f_only && accum_int >= 0x80 {
            self.err_span_(start_bpos,
                           self.pos,
                           "this form of character escape may only be used with characters in \
                            the range [\\x00-\\x7f]");
            valid = false;
        }

        match char::from_u32(accum_int) {
            Some(_) => valid,
            None => {
                let last_bpos = self.pos;
                self.err_span_(start_bpos, last_bpos, "invalid numeric character escape");
                false
            }
        }
    }

    /// Scan for a single (possibly escaped) byte or char
    /// in a byte, (non-raw) byte string, char, or (non-raw) string literal.
    /// `start` is the position of `first_source_char`, which is already consumed.
    ///
    /// Returns true if there was a valid char/byte, false otherwise.
    fn scan_char_or_byte(&mut self,
                         start: BytePos,
                         first_source_char: char,
                         ascii_only: bool,
                         delim: char)
                         -> bool
    {
        match first_source_char {
            '\\' => {
                // '\X' for some X must be a character constant:
                let escaped = self.ch;
                let escaped_pos = self.pos;
                self.bump();
                match escaped {
                    None => {}  // EOF here is an error that will be checked later.
                    Some(e) => {
                        return match e {
                            'n' | 'r' | 't' | '\\' | '\'' | '"' | '0' => true,
                            'x' => self.scan_byte_escape(delim, !ascii_only),
                            'u' => {
                                let valid = if self.ch_is('{') {
                                    self.scan_unicode_escape(delim) && !ascii_only
                                } else {
                                    let span = self.mk_sp(start, self.pos);
                                    let mut suggestion = "\\u{".to_owned();
                                    let mut err = self.sess.span_diagnostic.struct_span_err(
                                        span,
                                        "incorrect unicode escape sequence",
                                    );
                                    let mut i = 0;
                                    while let (Some(ch), true) = (self.ch, i < 6) {
                                        if ch.is_digit(16) {
                                            suggestion.push(ch);
                                            self.bump();
                                            i += 1;
                                        } else {
                                            break;
                                        }
                                    }
                                    if i != 0 {
                                        suggestion.push('}');
                                        err.span_suggestion_with_applicability(
                                            self.mk_sp(start, self.pos),
                                            "format of unicode escape sequences uses braces",
                                            suggestion,
                                            Applicability::MaybeIncorrect,
                                        );
                                    } else {
                                        err.span_help(
                                            span,
                                            "format of unicode escape sequences is `\\u{...}`",
                                        );
                                    }
                                    err.emit();
                                    false
                                };
                                if ascii_only {
                                    self.err_span_(start,
                                                   self.pos,
                                                   "unicode escape sequences cannot be used as a \
                                                    byte or in a byte string");
                                }
                                valid

                            }
                            '\n' if delim == '"' => {
                                self.consume_whitespace();
                                true
                            }
                            '\r' if delim == '"' && self.ch_is('\n') => {
                                self.consume_whitespace();
                                true
                            }
                            c => {
                                let pos = self.pos;
                                let mut err = self.struct_err_span_char(escaped_pos,
                                                                        pos,
                                                                        if ascii_only {
                                                                            "unknown byte escape"
                                                                        } else {
                                                                            "unknown character \
                                                                             escape"
                                                                        },
                                                                        c);
                                if e == '\r' {
                                    err.span_help(self.mk_sp(escaped_pos, pos),
                                                  "this is an isolated carriage return; consider \
                                                   checking your editor and version control \
                                                   settings");
                                }
                                if (e == '{' || e == '}') && !ascii_only {
                                    err.span_help(self.mk_sp(escaped_pos, pos),
                                                  "if used in a formatting string, curly braces \
                                                   are escaped with `{{` and `}}`");
                                }
                                err.emit();
                                false
                            }
                        }
                    }
                }
            }
            '\t' | '\n' | '\r' | '\'' if delim == '\'' => {
                let pos = self.pos;
                self.err_span_char(start,
                                   pos,
                                   if ascii_only {
                                       "byte constant must be escaped"
                                   } else {
                                       "character constant must be escaped"
                                   },
                                   first_source_char);
                return false;
            }
            '\r' => {
                if self.ch_is('\n') {
                    self.bump();
                    return true;
                } else {
                    self.err_span_(start,
                                   self.pos,
                                   "bare CR not allowed in string, use \\r instead");
                    return false;
                }
            }
            _ => {
                if ascii_only && first_source_char > '\x7F' {
                    let pos = self.pos;
                    self.err_span_(start,
                                   pos,
                                   "byte constant must be ASCII. Use a \\xHH escape for a \
                                    non-ASCII byte");
                    return false;
                }
            }
        }
        true
    }

    /// Scan over a `\u{...}` escape
    ///
    /// At this point, we have already seen the `\` and the `u`, the `{` is the current character.
    /// We will read a hex number (with `_` separators), with 1 to 6 actual digits,
    /// and pass over the `}`.
    fn scan_unicode_escape(&mut self, delim: char) -> bool {
        self.bump(); // past the {
        let start_bpos = self.pos;
        let mut valid = true;

        if let Some('_') = self.ch {
            // disallow leading `_`
            self.err_span_(self.pos,
                           self.next_pos,
                           "invalid start of unicode escape");
            valid = false;
        }

        let count = self.scan_digits(16, 16);

        if count > 6 {
            self.err_span_(start_bpos,
                           self.pos,
                           "overlong unicode escape (must have at most 6 hex digits)");
            valid = false;
        }

        loop {
            match self.ch {
                Some('}') => {
                    if valid && count == 0 {
                        self.err_span_(start_bpos,
                                       self.pos,
                                       "empty unicode escape (must have at least 1 hex digit)");
                        valid = false;
                    }
                    self.bump(); // past the ending `}`
                    break;
                },
                Some(c) => {
                    if c == delim {
                        self.err_span_(self.pos,
                                       self.pos,
                                       "unterminated unicode escape (needed a `}`)");
                        valid = false;
                        break;
                    } else if valid {
                        self.err_span_char(start_bpos,
                                           self.pos,
                                           "invalid character in unicode escape",
                                           c);
                        valid = false;
                    }
                },
                None => {
                    self.fatal_span_(start_bpos,
                                     self.pos,
                                     "unterminated unicode escape (found EOF)").raise();
                }
            }
            self.bump();
        }

        valid
    }

    /// Scan over a float exponent.
    fn scan_float_exponent(&mut self) {
        if self.ch_is('e') || self.ch_is('E') {
            self.bump();

            if self.ch_is('-') || self.ch_is('+') {
                self.bump();
            }

            if self.scan_digits(10, 10) == 0 {
                let mut err = self.struct_span_fatal(
                    self.pos, self.next_pos,
                    "expected at least one digit in exponent"
                );
                if let Some(ch) = self.ch {
                    // check for e.g., Unicode minus '−' (Issue #49746)
                    if unicode_chars::check_for_substitution(self, ch, &mut err) {
                        self.bump();
                        self.scan_digits(10, 10);
                    }
                }
                err.emit();
            }
        }
    }

    /// Check that a base is valid for a floating literal, emitting a nice
    /// error if it isn't.
    fn check_float_base(&mut self, start_bpos: BytePos, last_bpos: BytePos, base: usize) {
        match base {
            16 => {
                self.err_span_(start_bpos,
                               last_bpos,
                               "hexadecimal float literal is not supported")
            }
            8 => {
                self.err_span_(start_bpos,
                               last_bpos,
                               "octal float literal is not supported")
            }
            2 => {
                self.err_span_(start_bpos,
                               last_bpos,
                               "binary float literal is not supported")
            }
            _ => (),
        }
    }

    fn binop(&mut self, op: token::BinOpToken) -> token::Token {
        self.bump();
        if self.ch_is('=') {
            self.bump();
            token::BinOpEq(op)
        } else {
            token::BinOp(op)
        }
    }

    /// Return the next token from the string, advances the input past that
    /// token, and updates the interner
    fn next_token_inner(&mut self) -> Result<token::Token, ()> {
        let c = self.ch;

        if ident_start(c) {
            let (is_ident_start, is_raw_ident) =
                match (c.unwrap(), self.nextch(), self.nextnextch()) {
                    // r# followed by an identifier starter is a raw identifier.
                    // This is an exception to the r# case below.
                    ('r', Some('#'), x) if ident_start(x) => (true, true),
                    // r as in r" or r#" is part of a raw string literal.
                    // b as in b' is part of a byte literal.
                    // They are not identifiers, and are handled further down.
                    ('r', Some('"'), _) |
                    ('r', Some('#'), _) |
                    ('b', Some('"'), _) |
                    ('b', Some('\''), _) |
                    ('b', Some('r'), Some('"')) |
                    ('b', Some('r'), Some('#')) => (false, false),
                    _ => (true, false),
                };

            if is_ident_start {
                let raw_start = self.pos;
                if is_raw_ident {
                    // Consume the 'r#' characters.
                    self.bump();
                    self.bump();
                }

                let start = self.pos;
                self.bump();

                while ident_continue(self.ch) {
                    self.bump();
                }

                return Ok(self.with_str_from(start, |string| {
                    // FIXME: perform NFKC normalization here. (Issue #2253)
                    let ident = self.mk_ident(string);

                    if is_raw_ident && (ident.is_path_segment_keyword() ||
                                        ident.name == keywords::Underscore.name()) {
                        self.fatal_span_(raw_start, self.pos,
                            &format!("`r#{}` is not currently supported.", ident.name)
                        ).raise();
                    }

                    if is_raw_ident {
                        let span = self.mk_sp(raw_start, self.pos);
                        self.sess.raw_identifier_spans.borrow_mut().push(span);
                    }

                    token::Ident(ident, is_raw_ident)
                }));
            }
        }

        if is_dec_digit(c) {
            let num = self.scan_number(c.unwrap());
            let suffix = self.scan_optional_raw_name();
            debug!("next_token_inner: scanned number {:?}, {:?}", num, suffix);
            return Ok(token::Literal(num, suffix));
        }

        match c.expect("next_token_inner called at EOF") {
            // One-byte tokens.
            ';' => {
                self.bump();
                Ok(token::Semi)
            }
            ',' => {
                self.bump();
                Ok(token::Comma)
            }
            '.' => {
                self.bump();
                if self.ch_is('.') {
                    self.bump();
                    if self.ch_is('.') {
                        self.bump();
                        Ok(token::DotDotDot)
                    } else if self.ch_is('=') {
                        self.bump();
                        Ok(token::DotDotEq)
                    } else {
                        Ok(token::DotDot)
                    }
                } else {
                    Ok(token::Dot)
                }
            }
            '(' => {
                self.bump();
                Ok(token::OpenDelim(token::Paren))
            }
            ')' => {
                self.bump();
                Ok(token::CloseDelim(token::Paren))
            }
            '{' => {
                self.bump();
                Ok(token::OpenDelim(token::Brace))
            }
            '}' => {
                self.bump();
                Ok(token::CloseDelim(token::Brace))
            }
            '[' => {
                self.bump();
                Ok(token::OpenDelim(token::Bracket))
            }
            ']' => {
                self.bump();
                Ok(token::CloseDelim(token::Bracket))
            }
            '@' => {
                self.bump();
                Ok(token::At)
            }
            '#' => {
                self.bump();
                Ok(token::Pound)
            }
            '~' => {
                self.bump();
                Ok(token::Tilde)
            }
            '?' => {
                self.bump();
                Ok(token::Question)
            }
            ':' => {
                self.bump();
                if self.ch_is(':') {
                    self.bump();
                    Ok(token::ModSep)
                } else {
                    Ok(token::Colon)
                }
            }

            '$' => {
                self.bump();
                Ok(token::Dollar)
            }

            // Multi-byte tokens.
            '=' => {
                self.bump();
                if self.ch_is('=') {
                    self.bump();
                    Ok(token::EqEq)
                } else if self.ch_is('>') {
                    self.bump();
                    Ok(token::FatArrow)
                } else {
                    Ok(token::Eq)
                }
            }
            '!' => {
                self.bump();
                if self.ch_is('=') {
                    self.bump();
                    Ok(token::Ne)
                } else {
                    Ok(token::Not)
                }
            }
            '<' => {
                self.bump();
                match self.ch.unwrap_or('\x00') {
                    '=' => {
                        self.bump();
                        Ok(token::Le)
                    }
                    '<' => {
                        Ok(self.binop(token::Shl))
                    }
                    '-' => {
                        self.bump();
                        Ok(token::LArrow)
                    }
                    _ => {
                        Ok(token::Lt)
                    }
                }
            }
            '>' => {
                self.bump();
                match self.ch.unwrap_or('\x00') {
                    '=' => {
                        self.bump();
                        Ok(token::Ge)
                    }
                    '>' => {
                        Ok(self.binop(token::Shr))
                    }
                    _ => {
                        Ok(token::Gt)
                    }
                }
            }
            '\'' => {
                // Either a character constant 'a' OR a lifetime name 'abc
                let start_with_quote = self.pos;
                self.bump();
                let start = self.pos;

                // the eof will be picked up by the final `'` check below
                let c2 = self.ch.unwrap_or('\x00');
                self.bump();

                // If the character is an ident start not followed by another single
                // quote, then this is a lifetime name:
                if ident_start(Some(c2)) && !self.ch_is('\'') {
                    while ident_continue(self.ch) {
                        self.bump();
                    }
                    // lifetimes shouldn't end with a single quote
                    // if we find one, then this is an invalid character literal
                    if self.ch_is('\'') {
                        self.fatal_span_verbose(start_with_quote, self.next_pos,
                                String::from("character literal may only contain one codepoint"))
                            .raise();

                    }

                    // Include the leading `'` in the real identifier, for macro
                    // expansion purposes. See #12512 for the gory details of why
                    // this is necessary.
                    let ident = self.with_str_from(start, |lifetime_name| {
                        self.mk_ident(&format!("'{}", lifetime_name))
                    });

                    return Ok(token::Lifetime(ident));
                }

                let valid = self.scan_char_or_byte(start, c2, /* ascii_only */ false, '\'');

                if !self.ch_is('\'') {
                    let pos = self.pos;

                    loop {
                        self.bump();
                        if self.ch_is('\'') {
                            let start = self.src_index(start);
                            let end = self.src_index(self.pos);
                            self.bump();
                            let span = self.mk_sp(start_with_quote, self.pos);
                            self.sess.span_diagnostic
                                .struct_span_err(span,
                                                 "character literal may only contain one codepoint")
                                .span_suggestion_with_applicability(
                                    span,
                                    "if you meant to write a `str` literal, use double quotes",
                                    format!("\"{}\"", &self.src[start..end]),
                                    Applicability::MachineApplicable
                                ).emit();
                            return Ok(token::Literal(token::Str_(Symbol::intern("??")), None))
                        }
                        if self.ch_is('\n') || self.is_eof() || self.ch_is('/') {
                            // Only attempt to infer single line string literals. If we encounter
                            // a slash, bail out in order to avoid nonsensical suggestion when
                            // involving comments.
                            break;
                        }
                    }

                    self.fatal_span_verbose(start_with_quote, pos,
                        String::from("character literal may only contain one codepoint")).raise();
                }

                let id = if valid {
                    self.name_from(start)
                } else {
                    Symbol::intern("0")
                };

                self.bump(); // advance ch past token
                let suffix = self.scan_optional_raw_name();

                Ok(token::Literal(token::Char(id), suffix))
            }
            'b' => {
                self.bump();
                let lit = match self.ch {
                    Some('\'') => self.scan_byte(),
                    Some('"') => self.scan_byte_string(),
                    Some('r') => self.scan_raw_byte_string(),
                    _ => unreachable!(),  // Should have been a token::Ident above.
                };
                let suffix = self.scan_optional_raw_name();

                Ok(token::Literal(lit, suffix))
            }
            '"' => {
                let start_bpos = self.pos;
                let mut valid = true;
                self.bump();

                while !self.ch_is('"') {
                    if self.is_eof() {
                        let last_bpos = self.pos;
                        self.fatal_span_(start_bpos,
                                         last_bpos,
                                         "unterminated double quote string").raise();
                    }

                    let ch_start = self.pos;
                    let ch = self.ch.unwrap();
                    self.bump();
                    valid &= self.scan_char_or_byte(ch_start, ch, /* ascii_only */ false, '"');
                }
                // adjust for the ASCII " at the start of the literal
                let id = if valid {
                    self.name_from(start_bpos + BytePos(1))
                } else {
                    Symbol::intern("??")
                };
                self.bump();
                let suffix = self.scan_optional_raw_name();

                Ok(token::Literal(token::Str_(id), suffix))
            }
            'r' => {
                let start_bpos = self.pos;
                self.bump();
                let mut hash_count: u16 = 0;
                while self.ch_is('#') {
                    if hash_count == 65535 {
                        let bpos = self.next_pos;
                        self.fatal_span_(start_bpos,
                                         bpos,
                                         "too many `#` symbols: raw strings may be \
                                         delimited by up to 65535 `#` symbols").raise();
                    }
                    self.bump();
                    hash_count += 1;
                }

                if self.is_eof() {
                    self.fail_unterminated_raw_string(start_bpos, hash_count);
                } else if !self.ch_is('"') {
                    let last_bpos = self.pos;
                    let curr_char = self.ch.unwrap();
                    self.fatal_span_char(start_bpos,
                                         last_bpos,
                                         "found invalid character; only `#` is allowed \
                                         in raw string delimitation",
                                         curr_char).raise();
                }
                self.bump();
                let content_start_bpos = self.pos;
                let mut content_end_bpos;
                let mut valid = true;
                'outer: loop {
                    if self.is_eof() {
                        self.fail_unterminated_raw_string(start_bpos, hash_count);
                    }
                    // if self.ch_is('"') {
                    // content_end_bpos = self.pos;
                    // for _ in 0..hash_count {
                    // self.bump();
                    // if !self.ch_is('#') {
                    // continue 'outer;
                    let c = self.ch.unwrap();
                    match c {
                        '"' => {
                            content_end_bpos = self.pos;
                            for _ in 0..hash_count {
                                self.bump();
                                if !self.ch_is('#') {
                                    continue 'outer;
                                }
                            }
                            break;
                        }
                        '\r' => {
                            if !self.nextch_is('\n') {
                                let last_bpos = self.pos;
                                self.err_span_(start_bpos,
                                               last_bpos,
                                               "bare CR not allowed in raw string, use \\r \
                                                instead");
                                valid = false;
                            }
                        }
                        _ => (),
                    }
                    self.bump();
                }

                self.bump();
                let id = if valid {
                    self.name_from_to(content_start_bpos, content_end_bpos)
                } else {
                    Symbol::intern("??")
                };
                let suffix = self.scan_optional_raw_name();

                Ok(token::Literal(token::StrRaw(id, hash_count), suffix))
            }
            '-' => {
                if self.nextch_is('>') {
                    self.bump();
                    self.bump();
                    Ok(token::RArrow)
                } else {
                    Ok(self.binop(token::Minus))
                }
            }
            '&' => {
                if self.nextch_is('&') {
                    self.bump();
                    self.bump();
                    Ok(token::AndAnd)
                } else {
                    Ok(self.binop(token::And))
                }
            }
            '|' => {
                match self.nextch() {
                    Some('|') => {
                        self.bump();
                        self.bump();
                        Ok(token::OrOr)
                    }
                    _ => {
                        Ok(self.binop(token::Or))
                    }
                }
            }
            '+' => {
                Ok(self.binop(token::Plus))
            }
            '*' => {
                Ok(self.binop(token::Star))
            }
            '/' => {
                Ok(self.binop(token::Slash))
            }
            '^' => {
                Ok(self.binop(token::Caret))
            }
            '%' => {
                Ok(self.binop(token::Percent))
            }
            c => {
                let last_bpos = self.pos;
                let bpos = self.next_pos;
                let mut err = self.struct_fatal_span_char(last_bpos,
                                                          bpos,
                                                          "unknown start of token",
                                                          c);
                unicode_chars::check_for_substitution(self, c, &mut err);
                self.fatal_errs.push(err);

                Err(())
            }
        }
    }

    fn consume_whitespace(&mut self) {
        while is_pattern_whitespace(self.ch) && !self.is_eof() {
            self.bump();
        }
    }

    fn read_to_eol(&mut self) -> String {
        let mut val = String::new();
        while !self.ch_is('\n') && !self.is_eof() {
            val.push(self.ch.unwrap());
            self.bump();
        }

        if self.ch_is('\n') {
            self.bump();
        }

        val
    }

    fn read_one_line_comment(&mut self) -> String {
        let val = self.read_to_eol();
        assert!((val.as_bytes()[0] == b'/' && val.as_bytes()[1] == b'/') ||
                (val.as_bytes()[0] == b'#' && val.as_bytes()[1] == b'!'));
        val
    }

    fn consume_non_eol_whitespace(&mut self) {
        while is_pattern_whitespace(self.ch) && !self.ch_is('\n') && !self.is_eof() {
            self.bump();
        }
    }

    fn peeking_at_comment(&self) -> bool {
        (self.ch_is('/') && self.nextch_is('/')) || (self.ch_is('/') && self.nextch_is('*')) ||
        // consider shebangs comments, but not inner attributes
        (self.ch_is('#') && self.nextch_is('!') && !self.nextnextch_is('['))
    }

    fn scan_byte(&mut self) -> token::Lit {
        self.bump();
        let start = self.pos;

        // the eof will be picked up by the final `'` check below
        let c2 = self.ch.unwrap_or('\x00');
        self.bump();

        let valid = self.scan_char_or_byte(start,
                                           c2,
                                           // ascii_only =
                                           true,
                                           '\'');
        if !self.ch_is('\'') {
            // Byte offsetting here is okay because the
            // character before position `start` are an
            // ascii single quote and ascii 'b'.
            let pos = self.pos;
            self.fatal_span_verbose(start - BytePos(2),
                                    pos,
                                    "unterminated byte constant".to_string()).raise();
        }

        let id = if valid {
            self.name_from(start)
        } else {
            Symbol::intern("?")
        };
        self.bump(); // advance ch past token

        token::Byte(id)
    }

    #[inline]
    fn scan_byte_escape(&mut self, delim: char, below_0x7f_only: bool) -> bool {
        self.scan_hex_digits(2, delim, below_0x7f_only)
    }

    fn scan_byte_string(&mut self) -> token::Lit {
        self.bump();
        let start = self.pos;
        let mut valid = true;

        while !self.ch_is('"') {
            if self.is_eof() {
                let pos = self.pos;
                self.fatal_span_(start, pos, "unterminated double quote byte string").raise();
            }

            let ch_start = self.pos;
            let ch = self.ch.unwrap();
            self.bump();
            valid &= self.scan_char_or_byte(ch_start,
                                            ch,
                                            // ascii_only =
                                            true,
                                            '"');
        }

        let id = if valid {
            self.name_from(start)
        } else {
            Symbol::intern("??")
        };
        self.bump();

        token::ByteStr(id)
    }

    fn scan_raw_byte_string(&mut self) -> token::Lit {
        let start_bpos = self.pos;
        self.bump();
        let mut hash_count = 0;
        while self.ch_is('#') {
            if hash_count == 65535 {
                let bpos = self.next_pos;
                self.fatal_span_(start_bpos,
                                 bpos,
                                 "too many `#` symbols: raw byte strings may be \
                                 delimited by up to 65535 `#` symbols").raise();
            }
            self.bump();
            hash_count += 1;
        }

        if self.is_eof() {
            self.fail_unterminated_raw_string(start_bpos, hash_count);
        } else if !self.ch_is('"') {
            let pos = self.pos;
            let ch = self.ch.unwrap();
            self.fatal_span_char(start_bpos,
                                        pos,
                                        "found invalid character; only `#` is allowed in raw \
                                         string delimitation",
                                        ch).raise();
        }
        self.bump();
        let content_start_bpos = self.pos;
        let mut content_end_bpos;
        'outer: loop {
            match self.ch {
                None => {
                    self.fail_unterminated_raw_string(start_bpos, hash_count);
                }
                Some('"') => {
                    content_end_bpos = self.pos;
                    for _ in 0..hash_count {
                        self.bump();
                        if !self.ch_is('#') {
                            continue 'outer;
                        }
                    }
                    break;
                }
                Some(c) => {
                    if c > '\x7F' {
                        let pos = self.pos;
                        self.err_span_char(pos, pos, "raw byte string must be ASCII", c);
                    }
                }
            }
            self.bump();
        }

        self.bump();

        token::ByteStrRaw(self.name_from_to(content_start_bpos, content_end_bpos), hash_count)
    }
}

// This tests the character for the unicode property 'PATTERN_WHITE_SPACE' which
// is guaranteed to be forward compatible. http://unicode.org/reports/tr31/#R3
#[inline]
crate fn is_pattern_whitespace(c: Option<char>) -> bool {
    c.map_or(false, Pattern_White_Space)
}

#[inline]
fn in_range(c: Option<char>, lo: char, hi: char) -> bool {
    c.map_or(false, |c| lo <= c && c <= hi)
}

#[inline]
fn is_dec_digit(c: Option<char>) -> bool {
    in_range(c, '0', '9')
}

fn is_doc_comment(s: &str) -> bool {
    let res = (s.starts_with("///") && *s.as_bytes().get(3).unwrap_or(&b' ') != b'/') ||
              s.starts_with("//!");
    debug!("is {:?} a doc comment? {}", s, res);
    res
}

fn is_block_doc_comment(s: &str) -> bool {
    // Prevent `/**/` from being parsed as a doc comment
    let res = ((s.starts_with("/**") && *s.as_bytes().get(3).unwrap_or(&b' ') != b'*') ||
               s.starts_with("/*!")) && s.len() >= 5;
    debug!("is {:?} a doc comment? {}", s, res);
    res
}

fn ident_start(c: Option<char>) -> bool {
    let c = match c {
        Some(c) => c,
        None => return false,
    };

    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_' || (c > '\x7f' && c.is_xid_start())
}

fn ident_continue(c: Option<char>) -> bool {
    let c = match c {
        Some(c) => c,
        None => return false,
    };

    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' ||
    (c > '\x7f' && c.is_xid_continue())
}

#[inline]
fn char_at(s: &str, byte: usize) -> char {
    s[byte..].chars().next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    use ast::{Ident, CrateConfig};
    use symbol::Symbol;
    use syntax_pos::{BytePos, Span, NO_EXPANSION};
    use source_map::SourceMap;
    use errors;
    use feature_gate::UnstableFeatures;
    use parse::token;
    use std::io;
    use std::path::PathBuf;
    use diagnostics::plugin::ErrorMap;
    use rustc_data_structures::fx::FxHashSet;
    use rustc_data_structures::sync::Lock;
    use with_globals;
    fn mk_sess(sm: Lrc<SourceMap>) -> ParseSess {
        let emitter = errors::emitter::EmitterWriter::new(Box::new(io::sink()),
                                                          Some(sm.clone()),
                                                          false,
                                                          false);
        ParseSess {
            span_diagnostic: errors::Handler::with_emitter(true, false, Box::new(emitter)),
            unstable_features: UnstableFeatures::from_environment(),
            config: CrateConfig::default(),
            included_mod_stack: Lock::new(Vec::new()),
            source_map: sm,
            missing_fragment_specifiers: Lock::new(FxHashSet::default()),
            raw_identifier_spans: Lock::new(Vec::new()),
            registered_diagnostics: Lock::new(ErrorMap::new()),
            buffered_lints: Lock::new(vec![]),
        }
    }

    // open a string reader for the given string
    fn setup<'a>(sm: &SourceMap,
                 sess: &'a ParseSess,
                 teststr: String)
                 -> StringReader<'a> {
        let sf = sm.new_source_file(PathBuf::from(teststr.clone()).into(), teststr);
        StringReader::new(sess, sf, None)
    }

    #[test]
    fn t1() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            let mut string_reader = setup(&sm,
                                        &sh,
                                        "/* my source file */ fn main() { println!(\"zebra\"); }\n"
                                            .to_string());
            let id = Ident::from_str("fn");
            assert_eq!(string_reader.next_token().tok, token::Comment);
            assert_eq!(string_reader.next_token().tok, token::Whitespace);
            let tok1 = string_reader.next_token();
            let tok2 = TokenAndSpan {
                tok: token::Ident(id, false),
                sp: Span::new(BytePos(21), BytePos(23), NO_EXPANSION),
            };
            assert_eq!(tok1.tok, tok2.tok);
            assert_eq!(tok1.sp, tok2.sp);
            assert_eq!(string_reader.next_token().tok, token::Whitespace);
            // the 'main' id is already read:
            assert_eq!(string_reader.pos.clone(), BytePos(28));
            // read another token:
            let tok3 = string_reader.next_token();
            let tok4 = TokenAndSpan {
                tok: mk_ident("main"),
                sp: Span::new(BytePos(24), BytePos(28), NO_EXPANSION),
            };
            assert_eq!(tok3.tok, tok4.tok);
            assert_eq!(tok3.sp, tok4.sp);
            // the lparen is already read:
            assert_eq!(string_reader.pos.clone(), BytePos(29))
        })
    }

    // check that the given reader produces the desired stream
    // of tokens (stop checking after exhausting the expected vec)
    fn check_tokenization(mut string_reader: StringReader, expected: Vec<token::Token>) {
        for expected_tok in &expected {
            assert_eq!(&string_reader.next_token().tok, expected_tok);
        }
    }

    // make the identifier by looking up the string in the interner
    fn mk_ident(id: &str) -> token::Token {
        token::Token::from_ast_ident(Ident::from_str(id))
    }

    #[test]
    fn doublecolonparsing() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            check_tokenization(setup(&sm, &sh, "a b".to_string()),
                            vec![mk_ident("a"), token::Whitespace, mk_ident("b")]);
        })
    }

    #[test]
    fn dcparsing_2() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            check_tokenization(setup(&sm, &sh, "a::b".to_string()),
                            vec![mk_ident("a"), token::ModSep, mk_ident("b")]);
        })
    }

    #[test]
    fn dcparsing_3() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            check_tokenization(setup(&sm, &sh, "a ::b".to_string()),
                            vec![mk_ident("a"), token::Whitespace, token::ModSep, mk_ident("b")]);
        })
    }

    #[test]
    fn dcparsing_4() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            check_tokenization(setup(&sm, &sh, "a:: b".to_string()),
                            vec![mk_ident("a"), token::ModSep, token::Whitespace, mk_ident("b")]);
        })
    }

    #[test]
    fn character_a() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            assert_eq!(setup(&sm, &sh, "'a'".to_string()).next_token().tok,
                    token::Literal(token::Char(Symbol::intern("a")), None));
        })
    }

    #[test]
    fn character_space() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            assert_eq!(setup(&sm, &sh, "' '".to_string()).next_token().tok,
                    token::Literal(token::Char(Symbol::intern(" ")), None));
        })
    }

    #[test]
    fn character_escaped() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            assert_eq!(setup(&sm, &sh, "'\\n'".to_string()).next_token().tok,
                    token::Literal(token::Char(Symbol::intern("\\n")), None));
        })
    }

    #[test]
    fn lifetime_name() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            assert_eq!(setup(&sm, &sh, "'abc".to_string()).next_token().tok,
                    token::Lifetime(Ident::from_str("'abc")));
        })
    }

    #[test]
    fn raw_string() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            assert_eq!(setup(&sm, &sh, "r###\"\"#a\\b\x00c\"\"###".to_string())
                        .next_token()
                        .tok,
                    token::Literal(token::StrRaw(Symbol::intern("\"#a\\b\x00c\""), 3), None));
        })
    }

    #[test]
    fn literal_suffixes() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            macro_rules! test {
                ($input: expr, $tok_type: ident, $tok_contents: expr) => {{
                    assert_eq!(setup(&sm, &sh, format!("{}suffix", $input)).next_token().tok,
                            token::Literal(token::$tok_type(Symbol::intern($tok_contents)),
                                            Some(Symbol::intern("suffix"))));
                    // with a whitespace separator:
                    assert_eq!(setup(&sm, &sh, format!("{} suffix", $input)).next_token().tok,
                            token::Literal(token::$tok_type(Symbol::intern($tok_contents)),
                                            None));
                }}
            }

            test!("'a'", Char, "a");
            test!("b'a'", Byte, "a");
            test!("\"a\"", Str_, "a");
            test!("b\"a\"", ByteStr, "a");
            test!("1234", Integer, "1234");
            test!("0b101", Integer, "0b101");
            test!("0xABC", Integer, "0xABC");
            test!("1.0", Float, "1.0");
            test!("1.0e10", Float, "1.0e10");

            assert_eq!(setup(&sm, &sh, "2us".to_string()).next_token().tok,
                    token::Literal(token::Integer(Symbol::intern("2")),
                                    Some(Symbol::intern("us"))));
            assert_eq!(setup(&sm, &sh, "r###\"raw\"###suffix".to_string()).next_token().tok,
                    token::Literal(token::StrRaw(Symbol::intern("raw"), 3),
                                    Some(Symbol::intern("suffix"))));
            assert_eq!(setup(&sm, &sh, "br###\"raw\"###suffix".to_string()).next_token().tok,
                    token::Literal(token::ByteStrRaw(Symbol::intern("raw"), 3),
                                    Some(Symbol::intern("suffix"))));
        })
    }

    #[test]
    fn line_doc_comments() {
        assert!(is_doc_comment("///"));
        assert!(is_doc_comment("/// blah"));
        assert!(!is_doc_comment("////"));
    }

    #[test]
    fn nested_block_comments() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            let mut lexer = setup(&sm, &sh, "/* /* */ */'a'".to_string());
            match lexer.next_token().tok {
                token::Comment => {}
                _ => panic!("expected a comment!"),
            }
            assert_eq!(lexer.next_token().tok,
                    token::Literal(token::Char(Symbol::intern("a")), None));
        })
    }

    #[test]
    fn crlf_comments() {
        with_globals(|| {
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
            let sh = mk_sess(sm.clone());
            let mut lexer = setup(&sm, &sh, "// test\r\n/// test\r\n".to_string());
            let comment = lexer.next_token();
            assert_eq!(comment.tok, token::Comment);
            assert_eq!((comment.sp.lo(), comment.sp.hi()), (BytePos(0), BytePos(7)));
            assert_eq!(lexer.next_token().tok, token::Whitespace);
            assert_eq!(lexer.next_token().tok,
                    token::DocComment(Symbol::intern("/// test")));
        })
    }
}
