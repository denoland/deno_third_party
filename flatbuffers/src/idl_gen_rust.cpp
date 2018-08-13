/*
 * Copyright 2014 Google Inc. All rights reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// independent from idl_parser, since this code is not needed for most clients

#include <iostream> // cerr messages for logging warnings

#include "flatbuffers/code_generators.h"
#include "flatbuffers/flatbuffers.h"
#include "flatbuffers/idl.h"
#include "flatbuffers/util.h"

namespace flatbuffers {

// Pedantic warning free version of toupper().
inline char ToUpper(char c) { return static_cast<char>(::toupper(c)); }

static std::string GeneratedFileName(const std::string &path,
                                     const std::string &file_name) {
  return path + file_name + "_generated.rs";
}

bool TypeNeedsLifetime(const Type &type) {
  switch (type.base_type) {
    case BASE_TYPE_STRING: {
      return true;
    }
    case BASE_TYPE_VECTOR: {
      return true;
    }
    case BASE_TYPE_STRUCT: {
      return !(type.struct_def->fixed);
    }
    case BASE_TYPE_UNION: {
      return true;
    }
    default: {
      return false;
    }
  }
}

bool StructNeedsLifetime(const StructDef &struct_def) {
  return !struct_def.fixed;

  if (!struct_def.fixed) {
    return true;
  }
  for (auto it = struct_def.fields.vec.begin();
      it != struct_def.fields.vec.end(); ++it) {
    const auto &field = **it;
    if (field.nested_flatbuffer != NULL ) {
      return true;
    }
    switch (field.value.type.base_type) {
      case BASE_TYPE_STRING: return true;
      case BASE_TYPE_VECTOR: return true;
      case BASE_TYPE_STRUCT: return true;
      case BASE_TYPE_UNION: return true;
      default: ;
    }
  }
  return false;
}

namespace rust {

class RustGenerator : public BaseGenerator {
 public:
  RustGenerator(const Parser &parser, const std::string &path,
                const std::string &file_name)
      : BaseGenerator(parser, path, file_name, "", "::"),
        cur_name_space_(nullptr) {
    const char *keywords[] = {
      // currently-used keywords
      "as",
      "break",
      "const",
      "continue",
      "crate",
      "else",
      "enum",
      "extern",
      "false",
      "fn",
      "for",
      "if",
      "impl",
      "in",
      "let",
      "loop",
      "match",
      "mod",
      "move",
      "mut",
      "pub",
      "ref",
      "return",
      "Self",
      "self",
      "static",
      "struct",
      "super",
      "trait",
      "true",
      "type",
      "unsafe",
      "use",
      "where",
      "while",

      // future possible keywords
      "abstract",
      "alignof",
      "become",
      "box",
      "do",
      "final",
      "macro",
      "offsetof",
      "override",
      "priv",
      "proc",
      "pure",
      "sizeof",
      "typeof",
      "unsized",
      "virtual",
      "yield",

      // other terms we should not use
      "std",
      "usize",
      "isize",
      "u8",
      "i8",
      "u16",
      "i16",
      "u32",
      "i32",
      "u64",
      "i64",
      "f32",
      "f64",
      nullptr };
    for (auto kw = keywords; *kw; kw++) keywords_.insert(*kw);
  }

  void GenIncludeDependencies() {
    int num_includes = 0;
    for (auto it = parser_.native_included_files_.begin();
         it != parser_.native_included_files_.end(); ++it) {
      code_ += "// #include \"" + *it + "\"";
      num_includes++;
    }
    for (auto it = parser_.included_files_.begin();
         it != parser_.included_files_.end(); ++it) {
      if (it->second.empty()) continue;
      auto noext = flatbuffers::StripExtension(it->second);
      auto basename = flatbuffers::StripPath(noext);

      code_ += "// #include \"" + parser_.opts.include_prefix +
               (parser_.opts.keep_include_path ? noext : basename) +
               "_generated.rs\"";
      num_includes++;
    }
    if (num_includes) code_ += "";
  }

  std::string EscapeKeyword(const std::string &name) const {
    return keywords_.find(name) == keywords_.end() ? name : name + "_";
  }

  std::string Name(const Definition &def) const {
    return EscapeKeyword(def.name);
  }

  std::string Name(const EnumVal &ev) const { return EscapeKeyword(ev.name); }

  std::string WrapInNameSpace(const Definition &def) const {
    return WrapInNameSpace(def.defined_namespace, def.name);
  }
  std::string WrapInNameSpace(const Namespace *ns,
                              const std::string &name) const {
    if (CurrentNameSpace() == ns) return name;
    std::string prefix = GetRelativeNamespaceTraversal(CurrentNameSpace(), ns);
    return prefix + name;
    //std::string qualified_name = qualifying_start_;
    //for (auto it = ns->components.begin(); it != ns->components.end(); ++it)
    //  qualified_name += *it + qualifying_separator_;
    //return qualified_name + name;
  }

  // Iterate through all definitions we haven't generated code for (enums,
  // structs, and tables) and output them to a single file.
  bool generate() {
    code_.Clear();
    code_ += "// " + std::string(FlatBuffersGeneratedWarning()) + "\n\n";

    if (parser_.opts.include_dependence_headers) { GenIncludeDependencies(); }

    assert(!cur_name_space_);

    // Generate all code in their namespaces, once, because Rust does not
    // permit re-opening modules. TODO: O(n**2) -> O(n) with a dictionary.
    for (auto it = parser_.namespaces_.begin(); it != parser_.namespaces_.end();
         ++it) {
      const auto &ns = *it;

      // Generate code for all the enum declarations.
      for (auto it = parser_.enums_.vec.begin(); it != parser_.enums_.vec.end();
           ++it) {
        const auto &enum_def = **it;
        if (enum_def.defined_namespace != ns) { continue; }
        if (!enum_def.generated) {
          SetNameSpace(enum_def.defined_namespace);
          GenEnum(enum_def);
        }
      }

      // Generate code for all structs, then all tables.
      for (auto it = parser_.structs_.vec.begin();
           it != parser_.structs_.vec.end(); ++it) {
        const auto &struct_def = **it;
        if (struct_def.defined_namespace != ns) { continue; }
        if (struct_def.fixed && !struct_def.generated) {
          SetNameSpace(struct_def.defined_namespace);
          GenStruct(struct_def);
        }
      }
      for (auto it = parser_.structs_.vec.begin();
           it != parser_.structs_.vec.end(); ++it) {
        const auto &struct_def = **it;
        if (struct_def.defined_namespace != ns) { continue; }
        if (!struct_def.fixed && !struct_def.generated) {
          SetNameSpace(struct_def.defined_namespace);
          GenTable(struct_def);
        }
      }

      // Generate convenient global helper functions:
      if (parser_.root_struct_def_) {
        auto &struct_def = *parser_.root_struct_def_;
        if (struct_def.defined_namespace != ns) { continue; }
        SetNameSpace(struct_def.defined_namespace);
        auto name = Name(struct_def);
        //auto qualified_name = cur_name_space_->GetFullyQualifiedName(name);
        auto cpp_name = WrapInNameSpace(struct_def.defined_namespace, name);

        code_.SetValue("STRUCT_NAME", name);
        code_.SetValue("CPP_NAME", cpp_name);
        code_.SetValue("NULLABLE_EXT", NullableExtension());

        // The root datatype accessors:
        code_ += "#[inline]";
        code_ +=
            "pub fn GetRootAs{{STRUCT_NAME}}<'a>(buf: &'a [u8])"
            " -> {{CPP_NAME}}<'a> {{NULLABLE_EXT}} {";
        code_ += "  flatbuffers::get_root::<{{CPP_NAME}}<'a>>(buf)";
        code_ += "}";
        code_ += "";

        code_ += "#[inline]";
        code_ +=
            "pub fn GetSizePrefixedRootAs{{STRUCT_NAME}}<'a>(buf: &'a [u8])"
            " -> {{CPP_NAME}}<'a> {{NULLABLE_EXT}} {";
        code_ += "  flatbuffers::get_size_prefixed_root::<{{CPP_NAME}}<'a>>(buf)";
        code_ += "}";
        code_ += "";

        if (parser_.opts.mutable_buffer) {
          code_ += "#[inline]";
          code_ += "pub fn GetMutable{{STRUCT_NAME}}(buf: &[u8]) -> &{{STRUCT_NAME}} {";
          code_ += "  return flatbuffers::get_mutable_root::<&{{STRUCT_NAME}}>(buf);";
          code_ += "}";
          code_ += "";
        }

        if (parser_.file_identifier_.length()) {
          // Return the identifier
          code_ += "#[inline]";
          code_ += "pub fn {{STRUCT_NAME}}Identifier() -> &'static str {";
          code_ += "  return \"" + parser_.file_identifier_ + "\";";
          code_ += "}";
          code_ += "";

          // Check if a buffer has the identifier.
          code_ += "#[inline]";
          code_ += "pub fn {{STRUCT_NAME}}BufferHasIdentifier(buf: &[u8])"
                   " -> bool {";
          code_ += "  return flatbuffers::buffer_has_identifier(";
          code_ += "      buf, {{STRUCT_NAME}}Identifier(), false);";
          code_ += "}";
          code_ += "";
          code_ += "#[inline]";
          code_ += "pub fn {{STRUCT_NAME}}SizePrefixedBufferHasIdentifier(buf: &[u8])"
                   " -> bool {";
          code_ += "  return flatbuffers::buffer_has_identifier(";
          code_ += "      buf, {{STRUCT_NAME}}Identifier(), true);";
          code_ += "}";
          code_ += "";
        }

        if (parser_.file_extension_.length()) {
          // Return the extension
          code_ += "#[inline]";
          code_ += "pub fn {{STRUCT_NAME}}Extension() -> &'static str {";
          code_ += "  return \"" + parser_.file_extension_ + "\";";
          code_ += "}";
          code_ += "";
        }

        // Finish a buffer with a given root object:
        code_.SetValue("OFFSET_TYPELABEL", Name(struct_def) + "Offset");
        code_ += "#[inline]";
        code_ += "pub fn Finish{{STRUCT_NAME}}Buffer<'a, 'b>(";
        code_ += "    fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>,";
        code_ += "    root: flatbuffers::Offset<{{STRUCT_NAME}}<'a>>) {";
        if (parser_.file_identifier_.length()) {
          code_ += "  fbb.finish(root, Some({{STRUCT_NAME}}Identifier()));";
        } else {
          code_ += "  fbb.finish(root, None);";
        }
        code_ += "}";
        code_ += "";
        code_ += "#[inline]";
        code_ += "pub fn FinishSizePrefixed{{STRUCT_NAME}}Buffer<'a, 'b>(";
        code_ += "    fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>,";
        code_ += "    root: flatbuffers::Offset<{{STRUCT_NAME}}<'a>>) {";
        if (parser_.file_identifier_.length()) {
          code_ += "  fbb.finish_size_prefixed(root, Some({{STRUCT_NAME}}Identifier()));";
        } else {
          code_ += "  fbb.finish_size_prefixed(root, None);";
        }
        code_ += "}";
      }

    }
    if (cur_name_space_) SetNameSpace(nullptr);

    const auto file_path = GeneratedFileName(path_, file_name_);
    const auto final_code = code_.ToString();
    return SaveFile(file_path.c_str(), final_code, false);
  }

 private:
  CodeWriter code_;

  std::set<std::string> keywords_;

  // This tracks the current namespace so we can insert namespace declarations.
  const Namespace *cur_name_space_;

  const Namespace *CurrentNameSpace() const { return cur_name_space_; }

  // Translates a qualified name in flatbuffer text format to the same name in
  // the equivalent C++ namespace.
  static std::string TranslateNameSpace(const std::string &qualified_name) {
    std::string cpp_qualified_name = qualified_name;
    size_t start_pos = 0;
    while ((start_pos = cpp_qualified_name.find(".", start_pos)) !=
           std::string::npos) {
      cpp_qualified_name.replace(start_pos, 1, "::");
    }
    return cpp_qualified_name;
  }

  void GenComment(const std::vector<std::string> &dc, const char *prefix = "") {
    std::string text;
    ::flatbuffers::GenComment(dc, &text, nullptr, prefix);
    code_ += text + "\\";
  }

  // Return a C++ type from the table in idl.h
  std::string GenTypeBasic(const Type &type, bool user_facing_type) const {
    static const char *ctypename[] = {
    // clang-format off
    #define FLATBUFFERS_TD(ENUM, IDLTYPE, CTYPE, JTYPE, GTYPE, NTYPE, PTYPE, \
                           RTYPE) \
            #RTYPE,
        FLATBUFFERS_GEN_TYPES(FLATBUFFERS_TD)
    #undef FLATBUFFERS_TD
      // clang-format on
    };
    if (user_facing_type) {
      if (type.enum_def) return WrapInNameSpace(*type.enum_def);
      if (type.base_type == BASE_TYPE_BOOL) return "bool";
    }
    return ctypename[type.base_type];
  }

  std::string GenTypeBasicForRepr(const Type &type) const {
    static const char *ctypename[] = {
    // clang-format off
    #define FLATBUFFERS_TD(ENUM, IDLTYPE, CTYPE, JTYPE, GTYPE, NTYPE, PTYPE, \
                           RTYPE) \
            #RTYPE,
        FLATBUFFERS_GEN_TYPES(FLATBUFFERS_TD)
    #undef FLATBUFFERS_TD
      // clang-format on
    };
    if (type.base_type == BASE_TYPE_BOOL) return "u8";
    return ctypename[type.base_type];
  }

  // Return a C++ pointer type, specialized to the actual struct/table types,
  // and vector element types.
  std::string GenTypePointer(const Type &type, const std::string &lifetime) const {
    switch (type.base_type) {
      case BASE_TYPE_STRING: {
        //return "&str";
        return "flatbuffers::StringOffset";
        //return "flatbuffers::String<" + lifetime + ">";
      }
      case BASE_TYPE_VECTOR: {
        const auto type_name = GenTypeWire(type.VectorType(), "", lifetime, false);
        //return "flatbuffers::Vector<" + type_name + ">";
        return "&" + lifetime + "[" + type_name + "]";
        //return "flatbuffers::LabeledVectorUOffsetT<" + type_name + ">";
      }
      case BASE_TYPE_STRUCT: {
        //return WrapInNameSpace(*type.struct_def);
        std::string s;
        //s.append(lifetime);
        s.append(WrapInNameSpace(type.struct_def->defined_namespace,
                                 type.struct_def->name));
        if (StructNeedsLifetime(*type.struct_def)) {
          s.append("<" + lifetime + ">");
        } else {
          s.append("/* foo */");
        }
        return s;
      }
      case BASE_TYPE_UNION: {
        return "flatbuffers::UnionOffset";
      }
      default: {
        assert(false);
      }
      // fall through
      //default: { return "&" + lifetime + "flatbuffers::Void"; }
      //default: { return "flatbuffers::Void<" + lifetime + ">"; }
      //default: { return "flatbuffers::UnionOffset"; }
    }
  }

  // Return a C++ type for any type (scalar/pointer) specifically for
  // building a flatbuffer.
  std::string GenTypeWire(const Type &type, const char *postfix,
                          const std::string &lifetime,
                          bool user_facing_type) const {
    if (IsScalar(type.base_type)) {
      return GenTypeBasic(type, user_facing_type) + postfix;
    } else if (IsStruct(type)) {
      // TODO distinguish between struct and table
      //return "&'xxx" + GenTypePointer(type, lifetime);
      //return "&" + lifetime + GenTypePointer(type, lifetime);
      return GenTypePointer(type, lifetime);
      //return "&" + lifetime + " " + GenTypePointer(type, lifetime) + postfix;
    } else if (type.base_type == BASE_TYPE_UNION) {
      return "flatbuffers::Offset<" + GenTypePointer(type, lifetime) + ">" + postfix;
      //return "Option<flatbuffers::LabeledUOffsetT<" + GenTypePointer(type, lifetime) + ">>" + postfix;
    } else {
      return "flatbuffers::Offset<" + GenTypePointer(type, lifetime) + ">" + postfix;
    }
  }

  // Return a C++ type for any type (scalar/pointer) that reflects its
  // serialized size.
  std::string GenTypeSize(const Type &type) const {
    if (IsScalar(type.base_type)) {
      return GenTypeBasic(type, false);
    } else if (IsStruct(type)) {
      return GenTypePointer(type, "");
    } else {
      return "flatbuffers::UOffsetT";
    }
  }

  std::string NullableExtension() {
    return parser_.opts.gen_nullable ? " /* TODO _Nullable */ " : "";
  }

  static std::string NativeName(const std::string &name, const StructDef *sd,
                                const IDLOptions &opts) {
    return sd && !sd->fixed ? opts.object_prefix + name + opts.object_suffix
                            : name;
  }

  const std::string &PtrType(const FieldDef *field) {
    auto attr = field ? field->attributes.Lookup("cpp_ptr_type") : nullptr;
    return attr ? attr->constant : parser_.opts.cpp_object_api_pointer_type;
  }

  const std::string NativeString(const FieldDef *field) {
    auto attr = field ? field->attributes.Lookup("cpp_str_type") : nullptr;
    auto &ret = attr ? attr->constant : parser_.opts.cpp_object_api_string_type;
    if (ret.empty()) { return "std::string"; }
    return ret;
  }

  std::string GenTypeNativePtr(const std::string &type, const FieldDef *field,
                               bool is_constructor) {
    auto &ptr_type = PtrType(field);
    if (ptr_type != "naked") {
      return ptr_type + "<" + type + ">";
    } else if (is_constructor) {
      return "";
    } else {
      return type + " *";
    }
  }

  std::string GenPtrGet(const FieldDef &field) {
    auto &ptr_type = PtrType(&field);
    return ptr_type == "naked" ? "" : ".get()";
  }

  enum class FullElementType {
    Integer,
    Float,
    Bool,

    Struct,
    Table,

    EnumKey,
    UnionKey,

    UnionValue,

    String, // todo: bytestring
    VectorOfInteger, VectorOfFloat, VectorOfBool, VectorOfEnumKey, VectorOfStruct,
    VectorOfTable, VectorOfString, VectorOfUnionValue,
  };

  FullElementType GetFullElementType(const Type &type) const {
    // order matters for some of these conditionals
    if (type.base_type == BASE_TYPE_STRING) {
      return FullElementType::String;
    } else if (type.base_type == BASE_TYPE_STRUCT) {
      if (type.struct_def->fixed) {
        return FullElementType::Struct;
      } else {
        return FullElementType::Table;
      }
    } else if (type.base_type == BASE_TYPE_VECTOR) {
      switch (GetFullElementType(type.VectorType())) {
        case FullElementType::Integer: { return FullElementType::VectorOfInteger; }
        case FullElementType::Float: { return FullElementType::VectorOfFloat; }
        case FullElementType::Bool: { return FullElementType::VectorOfBool; }
        case FullElementType::Struct: { return FullElementType::VectorOfStruct; }
        case FullElementType::Table: { return FullElementType::VectorOfTable; }
        case FullElementType::String: { return FullElementType::VectorOfString; }
        case FullElementType::EnumKey: { return FullElementType::VectorOfEnumKey; }
        case FullElementType::UnionValue: { return FullElementType::VectorOfUnionValue; }
        default: { assert(false); }
      }
    } else if (type.enum_def != nullptr) {
      if (type.enum_def->is_union) {
        if (type.base_type == BASE_TYPE_UNION) {
          return FullElementType::UnionValue;
        } else if (type.base_type == BASE_TYPE_UTYPE) {
          return FullElementType::UnionKey;
        } else {
          assert(false);
        }
      } else {
        return FullElementType::EnumKey;
      }
    } else if (IsScalar(type.base_type)) {
      if (IsBool(type.base_type)) {
        return FullElementType::Bool;
      } else if (IsLong(type.base_type) || IsInteger(type.base_type)) {
        return FullElementType::Integer;
      } else if (IsFloat(type.base_type)) {
        return FullElementType::Float;
      } else {
        assert(false);
      }
    } else {
      assert(false);
    }
    assert(false);
    //TODO what is this? "or for an integral type derived from an enum."
  }

  enum class ContainerType { None, Vector, Enum, Union };
  ContainerType GetContainerType(const Type &type) const {
    if (type.base_type == BASE_TYPE_VECTOR) {
      return ContainerType::Vector;
    } else if (type.enum_def != nullptr) {
      if (type.enum_def->is_union) {
        return ContainerType::Union;
      } else {
        return ContainerType::Enum;
      }
    } else {
      return ContainerType::None;
    }
  }

  enum class ElementType { Struct, Table, Number, EnumValue, Bool, String, UnionMember, UnionEnumValue }; // TODO: bytestring
  ElementType GetElementType(const Type &origin_type) const {
    Type type = origin_type;
    if (GetContainerType(origin_type) == ContainerType::Vector) {
        type = origin_type.VectorType();
    }

    if (type.base_type == BASE_TYPE_STRUCT) {
      if (type.struct_def->fixed) {
        return ElementType::Struct;
      } else {
        return ElementType::Table;
      }
    } else if (type.base_type == BASE_TYPE_STRING) {
      return ElementType::String;
    } else if (type.enum_def && !type.enum_def->is_union) {
      return ElementType::EnumValue;
    } else if (type.enum_def && type.enum_def->is_union && type.base_type == BASE_TYPE_UNION) {
      return ElementType::UnionMember;
    } else if (type.enum_def && type.enum_def->is_union && type.base_type == BASE_TYPE_UTYPE) {
      return ElementType::UnionEnumValue;
    } else if (type.base_type == BASE_TYPE_UNION) {
      assert(false);
    } else if (type.base_type == BASE_TYPE_BOOL) {
      return ElementType::Bool;
    } else if (IsScalar(type.base_type)) {
      return ElementType::Number;
    } else {
      assert(false);
    }
  }

  std::string GenTypeNative(const Type &type, bool invector,
                            const FieldDef &field) {
    switch (type.base_type) {
      case BASE_TYPE_STRING: {
        return NativeString(&field);
      }
      case BASE_TYPE_VECTOR: {
        const auto type_name = GenTypeNative(type.VectorType(), true, field);
        if (type.struct_def &&
            type.struct_def->attributes.Lookup("native_custom_alloc")) {
          auto native_custom_alloc =
              type.struct_def->attributes.Lookup("native_custom_alloc");
          return "&[" + type_name + "," +
                 native_custom_alloc->constant + "<" + type_name + ">]";
        } else
          return "&[" + type_name + "]";
      }
      case BASE_TYPE_STRUCT: {
        auto type_name = WrapInNameSpace(*type.struct_def);
        if (IsStruct(type)) {
          auto native_type = type.struct_def->attributes.Lookup("native_type");
          if (native_type) { type_name = native_type->constant; }
          if (invector || field.native_inline) {
            return type_name;
          } else {
            return GenTypeNativePtr(type_name, &field, false);
          }
        } else {
          return GenTypeNativePtr(
              NativeName(type_name, type.struct_def, parser_.opts), &field,
              false);
        }
      }
      case BASE_TYPE_UNION: {
        return type.enum_def->name + "Union";
      }
      default: { return GenTypeBasic(type, true); }
    }
  }

  // Return a C++ type for any type (scalar/pointer) specifically for
  // using a flatbuffer.
  std::string GenTypeGet(const Type &type, const char *afterbasic,
                         const char *beforeptr, const char *afterptr,
                         bool user_facing_type) {
    if (IsScalar(type.base_type)) {
      return GenTypeBasic(type, user_facing_type) + afterbasic;
    } else {
      return beforeptr + GenTypePointer(type, "'a") + afterptr;
    }
  }

  //// Return a Rust++ type for any type (scalar/pointer) specifically for
  //// using a flatbuffer, including the relative namespace string path.
  //std::string GenTypeGet(const Type &type, const char *afterbasic,
  //                       const char *beforeptr, const char *afterptr,
  //                       bool user_facing_type, Namespace &ns) {
  //  if (IsScalar(type.base_type)) {
  //    return GenTypeBasic(type, user_facing_type) + afterbasic;
  //  } else {
  //    return beforeptr + GenTypePointer(type) + afterptr;
  //  }
  //}

  std::string GenEnumDecl(const EnumDef &enum_def) const {
    const IDLOptions &opts = parser_.opts;
    return (opts.scoped_enums ? "pub enum class " : "pub enum ") + Name(enum_def);
  }

  std::string GenEnumValDecl(const EnumDef &enum_def,
                             const std::string &enum_val) const {
    return enum_val;
  }

  std::string GetEnumValUse(const EnumDef &enum_def,
                            const EnumVal &enum_val) const {
    return Name(enum_def) + "::" + Name(enum_val);
    const IDLOptions &opts = parser_.opts;
    if (opts.scoped_enums) {
      return Name(enum_def) + "::" + Name(enum_val);
    } else if (opts.prefixed_enums) {
      return Name(enum_def) + "_" + Name(enum_val);
    } else {
      return Name(enum_val);
    }
  }

  std::string StripUnionType(const std::string &name) {
    return name.substr(0, name.size() - strlen(UnionTypeFieldSuffix()));
  }

  std::string GetUnionElement(const EnumVal &ev, bool wrap, bool actual_type,
                              bool native_type = false) {
    if (ev.union_type.base_type == BASE_TYPE_STRUCT) {
      auto name = actual_type ? ev.union_type.struct_def->name : Name(ev);
      return wrap ? WrapInNameSpace(
          ev.union_type.struct_def->defined_namespace, name)
                  : name;
    } else if (ev.union_type.base_type == BASE_TYPE_STRING) {
      return actual_type ? (native_type ? "std::string" : "&str")
                         : Name(ev);
    } else {
      assert(false);
      return Name(ev);
    }
  }

  std::string UnionVerifySignature(const EnumDef &enum_def) {
    return "pub fn Verify" + Name(enum_def) +
           "(verifier: &mut flatbuffers::Verifier, obj: &[u8], " +
           "type_: " + Name(enum_def) + ") -> bool";
  }

  std::string UnionVectorVerifySignature(const EnumDef &enum_def) {
    return "pub fn Verify" + Name(enum_def) + "Vector" +
           "(_verifier: &mut flatbuffers::Verifier, " +
           "values: &[flatbuffers::Offset<flatbuffers::Void>], " +
           "types: &[u8]) -> bool";
  }

  std::string UnionUnPackSignature(const EnumDef &enum_def, bool inclass) {
    return (inclass ? "static " : "") + std::string("void *") +
           (inclass ? "" : Name(enum_def) + "Union::") +
           "UnPack(const void *obj, " + Name(enum_def) +
           " type, const flatbuffers::resolver_function_t *resolver)";
  }

  std::string UnionPackSignature(const EnumDef &enum_def, bool inclass) {
    return "flatbuffers::Offset<flatbuffers::Void> " +
           (inclass ? "" : Name(enum_def) + "Union::") +
           "Pack(flatbuffers::FlatBufferBuilder &_fbb, " +
           "const flatbuffers::rehasher_function_t *_rehasher" +
           (inclass ? " = nullptr" : "") + ") const";
  }

  std::string TableCreateSignature(const StructDef &struct_def, bool predecl,
                                   const IDLOptions &opts) {
    return "flatbuffers::Offset<" + Name(struct_def) + "> Create" +
           Name(struct_def) + "(flatbuffers::FlatBufferBuilder &_fbb, const " +
           NativeName(Name(struct_def), &struct_def, opts) +
           " *_o, const flatbuffers::rehasher_function_t *_rehasher" +
           (predecl ? " = nullptr" : "") + ")";
  }

  std::string TablePackSignature(const StructDef &struct_def, bool inclass,
                                 const IDLOptions &opts) {
    return std::string(inclass ? "static " : "") + "flatbuffers::Offset<" +
           Name(struct_def) + "> " + (inclass ? "" : Name(struct_def) + "::") +
           "Pack(flatbuffers::FlatBufferBuilder &_fbb, " + "const " +
           NativeName(Name(struct_def), &struct_def, opts) + "* _o, " +
           "const flatbuffers::rehasher_function_t *_rehasher" +
           (inclass ? " = nullptr" : "") + ")";
  }

  std::string TableUnPackSignature(const StructDef &struct_def, bool inclass,
                                   const IDLOptions &opts) {
    return NativeName(Name(struct_def), &struct_def, opts) + " *" +
           (inclass ? "" : Name(struct_def) + "::") +
           "UnPack(const flatbuffers::resolver_function_t *_resolver" +
           (inclass ? " = nullptr" : "") + ") const";
  }

  std::string TableUnPackToSignature(const StructDef &struct_def, bool inclass,
                                     const IDLOptions &opts) {
    return "void " + (inclass ? "" : Name(struct_def) + "::") + "UnPackTo(" +
           NativeName(Name(struct_def), &struct_def, opts) + " *" +
           "_o, const flatbuffers::resolver_function_t *_resolver" +
           (inclass ? " = nullptr" : "") + ") const";
  }

  void GenMiniReflectPre(const StructDef *struct_def) {
    //code_.SetValue("NAME", struct_def->name);
    //code_ += "#[inline]";
    //code_ += "fn {{NAME}}TypeTable() -> &/*mut?*/ flatbuffers::TypeTable {}";
    //code_ += "";
  }

  void GenMiniReflect(const StructDef *struct_def, const EnumDef *enum_def) {
    code_.SetValue("NAME", struct_def ? struct_def->name : enum_def->name);
    code_.SetValue("SEQ_TYPE",
                   struct_def ? (struct_def->fixed ? "ST_STRUCT" : "ST_TABLE")
                              : (enum_def->is_union ? "ST_UNION" : "ST_ENUM"));
    auto num_fields =
        struct_def ? struct_def->fields.vec.size() : enum_def->vals.vec.size();
    code_.SetValue("NUM_FIELDS", NumToString(num_fields));
    std::vector<std::string> names;
    std::vector<Type> types;
    bool consecutive_enum_from_zero = true;
    if (struct_def) {
      for (auto it = struct_def->fields.vec.begin();
           it != struct_def->fields.vec.end(); ++it) {
        const auto &field = **it;
        names.push_back(Name(field));
        types.push_back(field.value.type);
      }
    } else {
      for (auto it = enum_def->vals.vec.begin(); it != enum_def->vals.vec.end();
           ++it) {
        const auto &ev = **it;
        names.push_back(Name(ev));
        types.push_back(enum_def->is_union ? ev.union_type
                                           : Type(enum_def->underlying_type));
        if (static_cast<int64_t>(it - enum_def->vals.vec.begin()) != ev.value) {
          consecutive_enum_from_zero = false;
        }
      }
    }
    std::string ts;
    std::vector<std::string> type_refs;
    for (auto it = types.begin(); it != types.end(); ++it) {
      auto &type = *it;
      if (!ts.empty()) ts += ",\n    ";
      auto is_vector = type.base_type == BASE_TYPE_VECTOR;
      auto bt = is_vector ? type.element : type.base_type;
      auto et = IsScalar(bt) || bt == BASE_TYPE_STRING
                    ? bt - BASE_TYPE_UTYPE + ET_UTYPE
                    : ET_SEQUENCE;
      int ref_idx = -1;
      std::string ref_name =
          type.struct_def
              ? WrapInNameSpace(*type.struct_def)
              : type.enum_def ? WrapInNameSpace(*type.enum_def) : "";
      if (!ref_name.empty()) {
        auto rit = type_refs.begin();
        for (; rit != type_refs.end(); ++rit) {
          if (*rit == ref_name) {
            ref_idx = static_cast<int>(rit - type_refs.begin());
            break;
          }
        }
        if (rit == type_refs.end()) {
          ref_idx = static_cast<int>(type_refs.size());
          type_refs.push_back(ref_name);
        }
      }
      ts += "{ flatbuffers::" + std::string(ElementaryTypeNames()[et]) + ", " +
            NumToString(is_vector) + ", " + NumToString(ref_idx) + " }";
    }
    std::string rs;
    for (auto it = type_refs.begin(); it != type_refs.end(); ++it) {
      if (!rs.empty()) rs += ",\n    ";
      rs += *it + "TypeTable";
    }
    std::string ns;
    for (auto it = names.begin(); it != names.end(); ++it) {
      if (!ns.empty()) ns += ",\n    ";
      ns += "\"" + *it + "\"";
    }
    std::string vs;
    if (enum_def && !consecutive_enum_from_zero) {
      for (auto it = enum_def->vals.vec.begin(); it != enum_def->vals.vec.end();
           ++it) {
        const auto &ev = **it;
        if (!vs.empty()) vs += ", ";
        vs += NumToString(ev.value);
      }
    } else if (struct_def && struct_def->fixed) {
      for (auto it = struct_def->fields.vec.begin();
           it != struct_def->fields.vec.end(); ++it) {
        const auto &field = **it;
        vs += NumToString(field.value.offset);
        vs += ", ";
      }
      vs += NumToString(struct_def->bytesize);
    }
    code_.SetValue("TYPES", ts);
    code_.SetValue("REFS", rs);
    code_.SetValue("NAMES", ns);
    code_.SetValue("VALUES", vs);
    code_ += "#[inline]";
    code_ += "pub fn {{NAME}}TypeTable() -> /*&mut?*/flatbuffers::TypeTable {";
    code_ += "  return flatbuffers::TypeTable{};";
    code_ += "  /* disable type table for now";
    if (num_fields) {
      code_ += "  static flatbuffers::TypeCode type_codes[] = {";
      code_ += "    {{TYPES}}";
      code_ += "  };";
    }
    if (!type_refs.empty()) {
      code_ += "  static flatbuffers::TypeFunction type_refs[] = {";
      code_ += "    {{REFS}}";
      code_ += "  };";
    }
    if (!vs.empty()) {
      code_ += "  static const int32_t values[] = { {{VALUES}} };";
    }
    auto has_names =
        num_fields && parser_.opts.mini_reflect == IDLOptions::kTypesAndNames;
    if (has_names) {
      code_ += "  static const char *names[] = {";
      code_ += "    {{NAMES}}";
      code_ += "  };";
    }
    code_ += "  static flatbuffers::TypeTable tt = {";
    code_ += std::string("    flatbuffers::{{SEQ_TYPE}}, {{NUM_FIELDS}}, ") +
             (num_fields ? "type_codes, " : "nullptr, ") +
             (!type_refs.empty() ? "type_refs, " : "nullptr, ") +
             (!vs.empty() ? "values, " : "nullptr, ") +
             (has_names ? "names" : "nullptr");
    code_ += "  };";
    code_ += "  return &tt;";
    code_ += "  */";
    code_ += "}";
    code_ += "";
  }

  // Generate an enum declaration,
  // an enum string lookup table,
  // an enum match function,
  // and an enum array of values
  void GenEnum(const EnumDef &enum_def) {
    code_.SetValue("ENUM_NAME", Name(enum_def));
    code_.SetValue("BASE_TYPE", GenTypeBasicForRepr(enum_def.underlying_type));
    code_.SetValue("SEP", "");

    GenComment(enum_def.doc_comment);
    code_ += "#[repr({{BASE_TYPE}})]";
    code_ += "#[derive(Clone, Copy, PartialEq, Debug)]";
    code_ += GenEnumDecl(enum_def) + "\\";
    //if (parser_.opts.scoped_enums) code_ += " : {{BASE_TYPE}}\\";
    code_ += " {";

    int64_t anyv = 0;
    const EnumVal *minv = nullptr, *maxv = nullptr;
    for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
         ++it) {
      const auto &ev = **it;

      GenComment(ev.doc_comment, "  ");
      code_.SetValue("KEY", GenEnumValDecl(enum_def, Name(ev)));
      code_.SetValue("VALUE", NumToString(ev.value));
      code_ += "{{SEP}}  {{KEY}} = {{VALUE}}\\";
      code_.SetValue("SEP", ",\n");

      minv = !minv || minv->value > ev.value ? &ev : minv;
      maxv = !maxv || maxv->value < ev.value ? &ev : maxv;
      anyv |= ev.value;
    }

    //// TODO: necessary?
    //if (parser_.opts.scoped_enums || parser_.opts.prefixed_enums) {
    //  assert(minv && maxv);

    //  code_.SetValue("SEP", ",\n");
    //  if (enum_def.attributes.Lookup("bit_flags")) {
    //    code_.SetValue("KEY", GenEnumValDecl(enum_def, "NONE"));
    //    code_.SetValue("VALUE", "0");
    //    code_ += "{{SEP}}  {{KEY}} = {{VALUE}}\\";

    //    code_.SetValue("KEY", GenEnumValDecl(enum_def, "ANY"));
    //    code_.SetValue("VALUE", NumToString(anyv));
    //    code_ += "{{SEP}}  {{KEY}} = {{VALUE}}\\";
    //  } else {  // MIN & MAX are useless for bit_flags
    //    code_.SetValue("KEY", GenEnumValDecl(enum_def, "MIN"));
    //    code_.SetValue("VALUE", GenEnumValDecl(enum_def, minv->name));
    //    code_ += "{{SEP}}  {{KEY}} = {{VALUE}}\\";

    //    code_.SetValue("KEY", GenEnumValDecl(enum_def, "MAX"));
    //    code_.SetValue("VALUE", GenEnumValDecl(enum_def, maxv->name));
    //    code_ += "{{SEP}}  {{KEY}} = {{VALUE}}\\";
    //  }
    //}
    code_ += "";
    code_ += "}";
    code_.SetValue("ENUM_NAME", Name(enum_def));

    //     code_ += "//#[repr({{BASE_TYPE}})]";
    //     code_ += "#[derive(Clone, Copy, PartialEq, Debug)]";
    //     code_ += GenEnumDecl(enum_def) + "Union\\";
    //     code_ += " {";

    //     int64_t anyv = 0;
    //     const EnumVal *minv = nullptr, *maxv = nullptr;
    //     for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //          ++it) {
    //       const auto &ev = **it;

    //       GenComment(ev.doc_comment, "  ");
    //       code_.SetValue("KEY", GenEnumValDecl(enum_def, Name(ev)));
    //       code_.SetValue("VALUE", NumToString(ev.value));
    //       code_ += "{{SEP}}  {{KEY}} = {{VALUE}}\\";
    //       code_.SetValue("SEP", ",\n");

    //       minv = !minv || minv->value > ev.value ? &ev : minv;
    //       maxv = !maxv || maxv->value < ev.value ? &ev : maxv;
    //       anyv |= ev.value;
    //     }

    //      code_ += "";
    //      code_ += "}";

    if (parser_.opts.scoped_enums && enum_def.attributes.Lookup("bit_flags")) {
      code_ += "DEFINE_BITMASK_OPERATORS({{ENUM_NAME}}, {{BASE_TYPE}})";
    }
    code_ += "";

    // Generate an array of all enumeration values
    auto num_fields = NumToString(enum_def.vals.vec.size());
    code_ += "const EnumValues{{ENUM_NAME}}:[{{ENUM_NAME}}; " +
              num_fields + "] = [";
    for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
         ++it) {
      const auto &ev = **it;
      auto value = GetEnumValUse(enum_def, ev);
      auto suffix = *it != enum_def.vals.vec.back() ? "," : "";
      code_ += "  " + value + suffix;
    }
    code_ += "];";
    code_ += "";

    // Generate a generate string table for enum values.
    // Problem is, if values are very sparse that could generate really big
    // tables. Ideally in that case we generate a map lookup instead, but for
    // the moment we simply don't output a table at all.
    auto range =
        enum_def.vals.vec.back()->value - enum_def.vals.vec.front()->value + 1;
    // Average distance between values above which we consider a table
    // "too sparse". Change at will.
    static const int kMaxSparseness = 5;
    if (range / static_cast<int64_t>(enum_def.vals.vec.size()) <
        kMaxSparseness) {
      code_ += "const EnumNames{{ENUM_NAME}}:[&'static str; " +
                NumToString(range) + "] = [";

      auto val = enum_def.vals.vec.front()->value;
      for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
           ++it) {
        const auto &ev = **it;
        while (val++ != ev.value) { code_ += "    \"\","; }
        auto suffix = *it != enum_def.vals.vec.back() ? "," : "";
        code_ += "    \"" + Name(ev) + "\"" + suffix;
      }
      code_ += "];";
      code_ += "";

      code_ += "pub fn EnumName{{ENUM_NAME}}(e: {{ENUM_NAME}}) -> &'static str {";

      code_ += "  let index: usize = e as usize\\";
      if (enum_def.vals.vec.front()->value) {
        auto vals = GetEnumValUse(enum_def, *enum_def.vals.vec.front());
        code_ += " - " + vals + " as usize\\";
      }
      code_ += ";";

      code_ += "  EnumNames{{ENUM_NAME}}[index]";
      code_ += "}";
      code_ += "";
    }

    if (enum_def.is_union) {
      // Generate tyoesafe offset(s) for unions
      code_.SetValue("NAME", Name(enum_def));
      code_ += "pub struct {{NAME}}UnionTableOffset {}";
    }

    // Skip this since we only use it (I think!) for object based api.
    //// Generate type traits for unions to map from a type to union enum value.
    //code_ += "pub trait {{ENUM_NAME}}Traits {";
    //code_ += "  const enum_value: usize = ;";
    //code_ += "}";
    //if (enum_def.is_union && !enum_def.uses_type_aliases) {
    //  for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //       ++it) {
    //    const auto &ev = **it;

    //    if (it == enum_def.vals.vec.begin()) {
    //    auto name = GetUnionElement(ev, true, true);
    //    auto value = GetEnumValUse(enum_def, ev);
    //    code_ += "impl {{ENUM_NAME}}Traits for " + name + " {";
    //    code_ += "  const enum_value: usize = " + value + ";";
    //    code_ += "}";
    //    code_ += "";
    //  }
    //}

    // Skip this since we only use it (I think!) for object based api.
    //if (parser_.opts.generate_object_based_api && enum_def.is_union) {
    //  // Generate a union type
    //  code_.SetValue("NAME", Name(enum_def));
    //  code_.SetValue("NONE",
    //                 GetEnumValUse(enum_def, *enum_def.vals.Lookup("NONE")));

    //  code_ += "struct {{NAME}}Union {";
    //  code_ += "  {{NAME}} type;";
    //  code_ += "  void *value;";
    //  code_ += "";
    //  code_ += "  {{NAME}}Union() : type({{NONE}}), value(nullptr) {}";
    //  code_ += "  {{NAME}}Union({{NAME}}Union&& u) FLATBUFFERS_NOEXCEPT :";
    //  code_ += "    type({{NONE}}), value(nullptr)";
    //  code_ += "    { std::swap(type, u.type); std::swap(value, u.value); }";
    //  code_ += "  {{NAME}}Union(const {{NAME}}Union &) FLATBUFFERS_NOEXCEPT;";
    //  code_ +=
    //      "  {{NAME}}Union &operator=(const {{NAME}}Union &u) "
    //      "FLATBUFFERS_NOEXCEPT";
    //  code_ +=
    //      "    { {{NAME}}Union t(u); std::swap(type, t.type); std::swap(value, "
    //      "t.value); return *this; }";
    //  code_ +=
    //      "  {{NAME}}Union &operator=({{NAME}}Union &&u) FLATBUFFERS_NOEXCEPT";
    //  code_ +=
    //      "    { std::swap(type, u.type); std::swap(value, u.value); return "
    //      "*this; }";
    //  code_ += "  ~{{NAME}}Union() { Reset(); }";
    //  code_ += "";
    //  code_ += "  void Reset();";
    //  code_ += "";
    //  if (!enum_def.uses_type_aliases) {
    //    code_ += "#ifndef FLATBUFFERS_CPP98_STL";
    //    code_ += "  template <typename T>";
    //    code_ += "  void Set(T&& val) {";
    //    code_ += "    Reset();";
    //    code_ +=
    //        "    type = {{NAME}}Traits<typename T::TableType>::enum_value;";
    //    code_ += "    if (type != {{NONE}}) {";
    //    code_ += "      value = new T(std::forward<T>(val));";
    //    code_ += "    }";
    //    code_ += "  }";
    //    code_ += "#endif  // FLATBUFFERS_CPP98_STL";
    //    code_ += "";
    //  }
    //  code_ += "  " + UnionUnPackSignature(enum_def, true) + ";";
    //  code_ += "  " + UnionPackSignature(enum_def, true) + ";";
    //  code_ += "";

    //  for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //       ++it) {
    //    const auto &ev = **it;
    //    if (!ev.value) { continue; }

    //    const auto native_type =
    //        NativeName(GetUnionElement(ev, true, true, true),
    //                   ev.union_type.struct_def, parser_.opts);
    //    code_.SetValue("NATIVE_TYPE", native_type);
    //    code_.SetValue("NATIVE_NAME", Name(ev));
    //    code_.SetValue("NATIVE_ID", GetEnumValUse(enum_def, ev));

    //    code_ += "  {{NATIVE_TYPE}} *As{{NATIVE_NAME}}() {";
    //    code_ += "    return type == {{NATIVE_ID}} ?";
    //    code_ += "      reinterpret_cast<{{NATIVE_TYPE}} *>(value) : nullptr;";
    //    code_ += "  }";

    //    code_ += "  const {{NATIVE_TYPE}} *As{{NATIVE_NAME}}() const {";
    //    code_ += "    return type == {{NATIVE_ID}} ?";
    //    code_ +=
    //        "      reinterpret_cast<const {{NATIVE_TYPE}} *>(value) : nullptr;";
    //    code_ += "  }";
    //  }
    //  code_ += "};";
    //  code_ += "";
    //}

    //if (enum_def.is_union) {
    //  code_ += UnionVerifySignature(enum_def) + ";";
    //  code_ += UnionVectorVerifySignature(enum_def) + ";";
    //  code_ += "";
    //}
  }

  void GenUnionPost(const EnumDef &enum_def) {
    return;
    // Generate a verifier function for this union that can be called by the
    // table verifier functions. It uses a switch case to select a specific
    // verifier function to call, this should be safe even if the union type
    // has been corrupted, since the verifiers will simply fail when called
    // on the wrong type.
    code_.SetValue("ENUM_NAME", Name(enum_def));

    code_ += "#[inline]";
    code_ += UnionVerifySignature(enum_def) + " {";
    code_ += "  match type_ {";
    for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
         ++it) {
      const auto &ev = **it;
      code_.SetValue("LABEL", GetEnumValUse(enum_def, ev));

      if (ev.value) {
        code_.SetValue("TYPE", GetUnionElement(ev, true, true));
        code_ += "    {{LABEL}} => {";
        auto getptr = "";
            //"      let x = obj.as_ptr() as *const {{TYPE}};";
        if (ev.union_type.base_type == BASE_TYPE_STRUCT) {
          if (ev.union_type.struct_def->fixed) {
            code_ += "      return true;";
          } else {
            code_ += getptr;
            code_ += "      if obj.len() != mem::size_of::<{{TYPE}}>() {";
            code_ += "          return false;";
            code_ += "      }";
            code_ += "      let x: &{{TYPE}} = unsafe {";
            code_ += "        &*(obj.as_ptr() as *const {{TYPE}})";
            code_ += "      };";
            code_ += "      return verifier.verify_table::<&{{TYPE}}>(x);";
          }
        } else if (ev.union_type.base_type == BASE_TYPE_STRING) {
          code_ += getptr;
          code_ += "      return verifier.Verify::<String>(x);";
        } else {
          assert(false);
        }
        code_ += "    }";
      } else {
        code_ += "    {{LABEL}} => {";
        code_ += "      return true;";  // "NONE" enum value.
        code_ += "    }";
      }
    }
    code_ += "  }";
    code_ += "}";
    code_ += "";

    code_ += "#[inline]";
    code_ += UnionVectorVerifySignature(enum_def) + " {";
    code_ += "  //if values.len() == 0 || types.len() == 0 {";
    code_ += "  //  return values.len() == types.len();";
    code_ += "  //}";
    code_ += "  if values.len() != types.len() { return false; }";
    code_ += "  //for _i in (0 as flatbuffers::UOffsetT)..values.len() {";
    code_ += "    //if !Verify" + Name(enum_def) + "(";
    code_ += "    //    verifier,  values.Get(i), types.GetEnum::<" +
             Name(enum_def) + ">(i)) {";
    code_ += "    //  return false;";
    code_ += "    //}";
    code_ += "  //}";
    code_ += "  return true;";
    code_ += "}";
    code_ += "";

    //if (parser_.opts.generate_object_based_api) {
    //  // Generate union Unpack() and Pack() functions.
    //  code_ += "inline " + UnionUnPackSignature(enum_def, false) + " {";
    //  code_ += "  switch (type) {";
    //  for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //       ++it) {
    //    const auto &ev = **it;
    //    if (!ev.value) { continue; }

    //    code_.SetValue("LABEL", GetEnumValUse(enum_def, ev));
    //    code_.SetValue("TYPE", GetUnionElement(ev, true, true));
    //    code_ += "    case {{LABEL}}: {";
    //    code_ += "      auto ptr = reinterpret_cast<const {{TYPE}} *>(obj);";
    //    if (ev.union_type.base_type == BASE_TYPE_STRUCT) {
    //      if (ev.union_type.struct_def->fixed) {
    //        code_ += "      return new " +
    //                 WrapInNameSpace(*ev.union_type.struct_def) + "(*ptr);";
    //      } else {
    //        code_ += "      return ptr->UnPack(resolver);";
    //      }
    //    } else if (ev.union_type.base_type == BASE_TYPE_STRING) {
    //      code_ += "      return new std::string(ptr->c_str(), ptr->size());";
    //    } else {
    //      assert(false);
    //    }
    //    code_ += "    }";
    //  }
    //  code_ += "    default: return nullptr;";
    //  code_ += "  }";
    //  code_ += "}";
    //  code_ += "";

    //  code_ += "inline " + UnionPackSignature(enum_def, false) + " {";
    //  code_ += "  switch (type) {";
    //  for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //       ++it) {
    //    auto &ev = **it;
    //    if (!ev.value) { continue; }

    //    code_.SetValue("LABEL", GetEnumValUse(enum_def, ev));
    //    code_.SetValue("TYPE",
    //                   NativeName(GetUnionElement(ev, true, true, true),
    //                              ev.union_type.struct_def, parser_.opts));
    //    code_.SetValue("NAME", GetUnionElement(ev, false, true));
    //    code_ += "    case {{LABEL}}: {";
    //    code_ += "      auto ptr = reinterpret_cast<const {{TYPE}} *>(value);";
    //    if (ev.union_type.base_type == BASE_TYPE_STRUCT) {
    //      if (ev.union_type.struct_def->fixed) {
    //        code_ += "      return _fbb.CreateStruct(*ptr).Union();";
    //      } else {
    //        code_ +=
    //            "      return Create{{NAME}}(_fbb, ptr, _rehasher).Union();";
    //      }
    //    } else if (ev.union_type.base_type == BASE_TYPE_STRING) {
    //      code_ += "      return _fbb.CreateString(*ptr).Union();";
    //    } else {
    //      assert(false);
    //    }
    //    code_ += "    }";
    //  }
    //  code_ += "    default: return 0;";
    //  code_ += "  }";
    //  code_ += "}";
    //  code_ += "";

    //  // Union copy constructor
    //  code_ +=
    //      "inline {{ENUM_NAME}}Union::{{ENUM_NAME}}Union(const "
    //      "{{ENUM_NAME}}Union &u) FLATBUFFERS_NOEXCEPT : type(u.type), "
    //      "value(nullptr) {";
    //  code_ += "  switch (type) {";
    //  for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //       ++it) {
    //    const auto &ev = **it;
    //    if (!ev.value) { continue; }
    //    code_.SetValue("LABEL", GetEnumValUse(enum_def, ev));
    //    code_.SetValue("TYPE",
    //                   NativeName(GetUnionElement(ev, true, true, true),
    //                              ev.union_type.struct_def, parser_.opts));
    //    code_ += "    case {{LABEL}}: {";
    //    bool copyable = true;
    //    if (ev.union_type.base_type == BASE_TYPE_STRUCT) {
    //      // Don't generate code to copy if table is not copyable.
    //      // TODO(wvo): make tables copyable instead.
    //      for (auto fit = ev.union_type.struct_def->fields.vec.begin();
    //           fit != ev.union_type.struct_def->fields.vec.end(); ++fit) {
    //        const auto &field = **fit;
    //        if (!field.deprecated && field.value.type.struct_def) {
    //          copyable = false;
    //          break;
    //        }
    //      }
    //    }
    //    if (copyable) {
    //      code_ +=
    //          "      value = new {{TYPE}}(*reinterpret_cast<{{TYPE}} *>"
    //          "(u.value));";
    //    } else {
    //      code_ += "      assert(false);  // {{TYPE}} not copyable.";
    //    }
    //    code_ += "      break;";
    //    code_ += "    }";
    //  }
    //  code_ += "    default:";
    //  code_ += "      break;";
    //  code_ += "  }";
    //  code_ += "}";
    //  code_ += "";

    //  // Union Reset() function.
    //  code_.SetValue("NONE",
    //                 GetEnumValUse(enum_def, *enum_def.vals.Lookup("NONE")));

    //  code_ += "inline void {{ENUM_NAME}}Union::Reset() {";
    //  code_ += "  switch (type) {";
    //  for (auto it = enum_def.vals.vec.begin(); it != enum_def.vals.vec.end();
    //       ++it) {
    //    const auto &ev = **it;
    //    if (!ev.value) { continue; }
    //    code_.SetValue("LABEL", GetEnumValUse(enum_def, ev));
    //    code_.SetValue("TYPE",
    //                   NativeName(GetUnionElement(ev, true, true, true),
    //                              ev.union_type.struct_def, parser_.opts));
    //    code_ += "    case {{LABEL}}: {";
    //    code_ += "      auto ptr = reinterpret_cast<{{TYPE}} *>(value);";
    //    code_ += "      delete ptr;";
    //    code_ += "      break;";
    //    code_ += "    }";
    //  }
    //  code_ += "    default: break;";
    //  code_ += "  }";
    //  code_ += "  value = nullptr;";
    //  code_ += "  type = {{NONE}};";
    //  code_ += "}";
    //  code_ += "";
    //}
  }

  // Generates a value with optionally a cast applied if the field has a
  // different underlying type from its interface type (currently only the
  // case for enums. "from" specify the direction, true meaning from the
  // underlying type to the interface type.
  std::string GenUnderlyingCast(const FieldDef &field, bool from,
                                const std::string &val) {
    if (from && field.value.type.base_type == BASE_TYPE_BOOL) {
      return val + " != 0";
    } else if ((field.value.type.enum_def &&
                IsScalar(field.value.type.base_type)) ||
               field.value.type.base_type == BASE_TYPE_BOOL) {
      // TODO(rw): handle enums in other namespaces
      if (from) {
        //return "EnumValues" + GenTypeBasic(field.value.type, from) + "[" + val + " as usize]";
        return "unsafe { ::std::mem::transmute(" + val + ") }";
      } else {
        return val + " as " + GenTypeBasic(field.value.type, from);
      }
    } else {
      return val;
    }
  }

  std::string GenFieldOffsetName(const FieldDef &field) {
    std::string uname = Name(field);
    std::transform(uname.begin(), uname.end(), uname.begin(), ToUpper);
    return "VT_" + uname;
  }

  void GenFullyQualifiedNameGetter(const StructDef &struct_def,
                                   const std::string &name) {
    if (!parser_.opts.generate_name_strings) { return; }
    auto fullname = struct_def.defined_namespace->GetFullyQualifiedName(name);
    code_.SetValue("NAME", fullname);
    code_.SetValue("CONSTEXPR", "FLATBUFFERS_CONSTEXPR");
    code_ += "  static {{CONSTEXPR}} const char *GetFullyQualifiedName() {";
    code_ += "    return \"{{NAME}}\";";
    code_ += "  }";
  }

  std::string GetRelativeNamespaceTraversal(const Namespace *src,
                                            const Namespace *dst) const {
    // calculate the path needed to reference dst from src.
    // example: f(A::B::C, A::B::C) -> n/a
    // example: f(A::B::C, A::B)    -> super::
    // example: f(A::B::C, A::B::D) -> super::D
    // example: f(A::B::C, A)       -> super::super::
    // example: f(A::B::C, D)       -> super::super::super::D
    // example: f(A::B::C, D::E)    -> super::super::super::D::E
    // example: f(A, D::E)          -> super::D::E
    // does not include leaf object (typically a struct type).
    //
    size_t i = 0;
    std::stringstream stream;

    auto s = src->components.begin();
    auto d = dst->components.begin();
    while(true) {
      if (s == src->components.end()) { break; }
      if (d == dst->components.end()) { break; }
      if (*s != *d) { break; }
      s++;
      d++;
      i++;
    }

    for (; s != src->components.end(); s++) {
      stream << "super::";
    }
    for (; d != dst->components.end(); d++) {
      stream << *d + "::";
    }
    return stream.str();
  }

  std::string GenDefaultConstant(const FieldDef &field) {
    //assert(false);
    return field.value.type.base_type == BASE_TYPE_FLOAT
               ? field.value.constant + ""
               : field.value.constant;
  }

  std::string GetDefaultScalarValueOld(const FieldDef &field) {
    assert(false);
    if (field.value.type.enum_def && IsScalar(field.value.type.base_type)) {
      auto ev = field.value.type.enum_def->ReverseLookup(
          StringToInt(field.value.constant.c_str()), false);
      if (ev) {
        return "/* A */" + WrapInNameSpace(field.value.type.enum_def->defined_namespace,
                               GetEnumValUse(*field.value.type.enum_def, *ev));
      } else {
        return "/* B */" + GenUnderlyingCast(field, true, field.value.constant);
      }
    } else if (field.value.type.base_type == BASE_TYPE_BOOL) {
      return field.value.constant == "0" ? "false" : "true";
    } else if (IsScalar(field.value.type.base_type)) {
      return "/* C */" + GenDefaultConstant(field);
    } else if (IsStruct(field.value.type)) {
      //return "/* D */ flatbuffers::LabeledUOffsetT::new(0)";
      //return "/* D */ None";
      return "/* D */ None";// + WrapInRelativeNameSpace(field.value.type.struct_def->defined_namespace,
                              ;//                     field.value.type.struct_def->name) + "::new()";
    } else {
      //return "/* E */ flatbuffers::LabeledUOffsetT::new(0)";
      return "/* E */ None";
    }
  }

  std::string GetDefaultScalarValue(const FieldDef &field) {
    switch (GetFullElementType(field.value.type)) {
      case FullElementType::Integer: { return GenDefaultConstant(field); }
      case FullElementType::Float: { return GenDefaultConstant(field); }
      case FullElementType::Bool: { return field.value.constant == "0" ? "false" : "true"; }
      case FullElementType::UnionKey:
      case FullElementType::EnumKey: {
        auto ev = field.value.type.enum_def->ReverseLookup(
            StringToInt(field.value.constant.c_str()), false);
        assert(ev);
        return WrapInNameSpace(field.value.type.enum_def->defined_namespace,
                               GetEnumValUse(*field.value.type.enum_def, *ev));
      }

      default: { return "None"; }
    }
  }

  // Note: we could make all inputs be an Option, as well as all outputs.
  // But the UX of Flatbuffers is that the user doesn't get to know if the value is default or not.
  std::string GenBuilderArgsDefnType(const FieldDef &field, const std::string lifetime) {
    //assert(false, "note to self: use real lifetimes for written objects--just give the returned offsets a lifetime compatible with the builder, not the original thing. then the offset can be dereferenced to read (or mutate?) the original object.");
    const Type& type = field.value.type;

    switch (GetFullElementType(field.value.type)) {
      case FullElementType::Integer:
      case FullElementType::Float:
      case FullElementType::Bool: {
        const auto typname = GenTypeBasic(type, false);
        return typname;
      }
      case FullElementType::Struct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<&" + lifetime + " " + typname + ">";
      }
      case FullElementType::Table: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<flatbuffers::Offset<&" + lifetime + " " + typname + "<" + lifetime + ">>>";
      }
      case FullElementType::String: {
        return "Option<flatbuffers::Offset<&" + lifetime + " str>>";
        //return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", u8>>>";
        //return "Option<flatbuffers::Offset<flatbuffers::StringOffset>>";
      }
      case FullElementType::EnumKey:
      case FullElementType::UnionKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return typname;
      }
      case FullElementType::UnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "Option<flatbuffers::Offset<" + typname + "UnionTableOffset>>";
      }

      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        //const auto basetype = GenTypeBasic(type.VectorType(), false);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ",  " + typname + ">>>";
      }
      case FullElementType::VectorOfBool: {
        const auto typname = GenTypeBasic(type, false);
        //const auto basetype = GenTypeBasic(type.VectorType(), false);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", bool>>>";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        //const auto basetype = GenTypeBasic(type.VectorType(), false);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>>";
      }
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>>";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsU32Offset<" + typname + "<" + lifetime + ">>>>>";
      }
      case FullElementType::VectorOfString: {
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsU32Offset<&" + lifetime + " str>>>>";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def) + "UnionTableOffset";
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", &" + lifetime + " Into<" + typname + "<" + lifetime + ">>>>>";
      }
    }
  }

  std::string GenBuilderArgsDefaultValue(const FieldDef &field) {
      return GetDefaultScalarValue(field);
  }
  std::string GenBuilderAddFuncDefaultValue(const FieldDef &field) {
    switch (GetFullElementType(field.value.type)) {

      case FullElementType::UnionKey:
      case FullElementType::EnumKey: {
        const std::string basetype = GenTypeBasic(field.value.type, false);
        return GetDefaultScalarValue(field) + " as " + basetype;
      }

      default: { return GetDefaultScalarValue(field); }
      //case FullElementType::Integer:
      //case FullElementType::Float:
      //case FullElementType::Bool: { return GetDefaultScalarValue(field); }
      //default: { return "None"; }
    }
  }

  std::string GenBuilderArgsAddFuncType(const FieldDef &field, const std::string lifetime) {
    const Type& type = field.value.type;

    switch (GetFullElementType(field.value.type)) {
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsU32Offset<" + typname + "<" + lifetime + ">>>>";
      }
      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        //const auto typname = GenTypeBasic(type, false);
        const auto typname = GenTypeBasic(type.VectorType(), false);
        //return "flatbuffers::Vector<" + typname + "<" + basetype + ">>";
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
      }
      case FullElementType::VectorOfBool: {
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", bool>>";
      }
      case FullElementType::VectorOfString: {
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsU32Offset<&" + lifetime + " str>>>";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
        //return typname;
        //const auto typname = GenTypeBasic(type, false);
        //const auto basetype = GenTypeBasic(type.VectorType(), false);
        //return "flatbuffers::VectorLabeledUOffsetT<" + typname + "<" + basetype + ">>";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::Offset<" + typname + ">>>";
        //const auto typname = WrapInNameSpace(*type.enum_def);
        //return typname;
      }
      case FullElementType::EnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return typname;
      }
      case FullElementType::Struct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "&" + lifetime + " " + typname + "";
      }
      case FullElementType::Table: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "flatbuffers::Offset<&" + lifetime + " " + typname + "<" + lifetime + ">>";
      }
      case FullElementType::Integer:
      case FullElementType::Float: {
        const auto typname = GenTypeBasic(type, false);
        //return "Option<" + typname + ">";
        return typname;
      }
      case FullElementType::Bool: {
        return "bool";
      }
      case FullElementType::String: {
        //return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", u8>>";
        return "flatbuffers::Offset<&" + lifetime + " str>";
      }
      case FullElementType::UnionKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return typname;
      }
      case FullElementType::UnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "flatbuffers::Offset<" + typname + "UnionTableOffset>";
      }
    }
  }

  std::string GenBuilderArgsAddFuncBody(const FieldDef &field) {
    const Type& type = field.value.type;

    switch (GetFullElementType(field.value.type)) {
          case FullElementType::Integer:
          case FullElementType::Float: {
            const auto typname = GenTypeWire(field.value.type, "", "", false);
            return "self.fbb_.push_slot_scalar::<" + typname + ">";
          }
          case FullElementType::Bool: {
            return "self.fbb_.push_slot_scalar::<bool>";
          }

          case FullElementType::Struct: {
            const auto typname = GenTypeWire(field.value.type, "", "", false);
            return "self.fbb_.push_slot_struct::<" + typname + ">";
          }
          case FullElementType::Table: {
            const auto typname = WrapInNameSpace(*type.struct_def);
            return "self.fbb_.push_slot_offset_relative::<&" + typname + ">";
          }

          case FullElementType::EnumKey:
          case FullElementType::UnionKey: {
            const auto underlying_typname = GenTypeBasic(type, false);
            return "self.fbb_.push_slot_scalar::<" + underlying_typname + ">";
          }

          case FullElementType::UnionValue:
          case FullElementType::String:
          case FullElementType::VectorOfInteger:
          case FullElementType::VectorOfFloat:
          case FullElementType::VectorOfBool:
          case FullElementType::VectorOfEnumKey:
          case FullElementType::VectorOfStruct:
          case FullElementType::VectorOfTable:
          case FullElementType::VectorOfString:
          case FullElementType::VectorOfUnionValue: {
            return "self.fbb_.push_slot_offset_relative";
          }
        }

    //      case FullElementType::VectorOfStruct: {
    //    return "self.fbb_.push_vector_todo";
    //        const auto typname = WrapInNameSpace(*type.struct_def);
    //        //return "Option<flatbuffers::VectorLabeledUOffsetT<&" + lifetime + " " + typname + ">>";
    //      }
    //      case FullElementType::Table: {
    //        const auto typname = WrapInNameSpace(*type.struct_def);
    //        return "flatbuffers::VectorLabeledUOffsetT<" + typname + "<'a>>";
    //      }
    //      case FullElementType::Number: {
    //        const auto typname = GenTypeBasic(type, false);
    //        const auto basetype = GenTypeBasic(type.VectorType(), false);
    //        return "flatbuffers::VectorLabeledUOffsetT<" + typname + "<" + basetype + ">>";
    //      }
    //      case FullElementType::Bool: {
    //        const auto typname = GenTypeBasic(type, false);
    //        return "flatbuffers::VectorLabeledUOffsetT<bool>";
    //      }
    //      case FullElementType::String: {
    //        return "flatbuffers::VectorLabeledUOffsetT<StringOffset>";
    //      }
    //      case FullElementType::EnumValue: {
    //        const auto typname = WrapInNameSpace(*type.enum_def);
    //        return typname;
    //      }
    //      case FullElementType::UnionEnumValue: {
    //        const auto typname = WrapInNameSpace(*type.enum_def);
    //        return typname;
    //      }
    //      case FullElementType::UnionMember: {
    //        const auto typname = WrapInNameSpace(*type.enum_def);
    //        return typname + "_UnionOffset";
    //      }
    //    }
    //  case ContainerType::Enum: {
    //    const auto underlying_typname = GenTypeWire(field.value.type, "", "", false);
    //    return "self.fbb_.push_slot_scalar::<" + underlying_typname + ">";
    //    //return "self.fbb_.push_enum_todo";
    //    ////const auto typname = GenTypeBasic(type, false);
    //    //const auto typname = WrapInNameSpace(*type.enum_def);
    //    ////return "Option<" + typname + ">";
    //    //return typname;
    //  }
    //}
  }

  std::string GenBuilderArgsAddFuncFieldCast(const FieldDef &field) {
    const Type& type = field.value.type;

    const auto ct = GetContainerType(type);
    const auto et = GetElementType(type);

    if (ct == ContainerType::Union && et == ElementType::UnionEnumValue) {
      return " as " + GenTypeBasic(type, false);
    }
    if (ct == ContainerType::Enum && et == ElementType::EnumValue) {
      return " as " + GenTypeBasic(type, false);
    }
    return "";
  }

  std::string GenTableAccessorFuncReturnType(const FieldDef &field,
                                             const std::string lifetime) {
    const Type& type = field.value.type;

    switch (GetFullElementType(field.value.type)) {
      case FullElementType::Integer:
      case FullElementType::Float: {
        const auto typname = GenTypeBasic(type, false);
        return typname;
      }
      case FullElementType::Bool: {
        return "bool";
      }
      case FullElementType::Struct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<&" + lifetime + " " + typname + ">";
      }
      case FullElementType::Table: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<" + typname + "<" + lifetime + ">>";
      }
      case FullElementType::EnumKey:
      case FullElementType::UnionKey: { const auto typname = WrapInNameSpace(*type.enum_def); return typname; }

      case FullElementType::UnionValue: {
        //const auto typname = WrapInNameSpace(*type.enum_def) + "_UnionEnum";
        //return "Option<" + typname + "UnionTableOffset>";
        //return "Option<flatbuffers::Vector<u8>>";
        //return "Option<flatbuffers::Table<" + lifetime + ">>";
        //return "Option<" + typename flatbuffers::Table<" + lifetime + ">>";
        return "Option<flatbuffers::Table<" + lifetime + ">>";
      }
      case FullElementType::String: {
         //return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", u8>>>";// + lifetime + ">>>";
         //return "Option<flatbuffers::FBString<" + lifetime + ">>";
         return "Option<&" + lifetime + " str>";
      }
      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        //return "Option<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
        return "Option<&" + lifetime + " [" + typname + "]>";
      }
      case FullElementType::VectorOfBool: {
        //return "Option<flatbuffers::Vector<" + lifetime + ", bool>>";
        return "Option<&" + lifetime + " [bool]>";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        //return "Option<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
        return "Option<&" + lifetime + " [" + typname + "]>";
      }
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        //return "Option<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
        return "Option<&" + lifetime + " [" + typname + "]>";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        //return "Option<flatbuffers::Vector<" + lifetime + ", flatbuffers::Offset<" + typname + "<" + lifetime + ">>>>";
        return "Option<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<" + typname + "<" + lifetime + ">>>>";
      }
      case FullElementType::VectorOfString: {
        //return "Option<flatbuffers::Vector<" + lifetime + ", &" + lifetime + " flatbuffers::String<" + lifetime + ">>>";
        return "Option<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<&" + lifetime + " str>>>";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def) + "UnionTableOffset";
        return "Option<flatbuffers::Vector<" + lifetime + ", &" + lifetime + " Into<" + typname + "<" + lifetime + ">>>>";
      }
    }
  }

  std::string GenTableAccessorFuncBody(const FieldDef &field,
                                       const std::string lifetime,
                                       const std::string offset_prefix) {
    //const std::string member = Name(field) + "_";
    const std::string offset_name = offset_prefix + "::" + GenFieldOffsetName(field);
    const Type& type = field.value.type;

    switch (GetFullElementType(field.value.type)) {
      case FullElementType::Integer:
      case FullElementType::Float:
      case FullElementType::Bool: {
        const auto typname = GenTypeBasic(type, false);
        //const auto typname = WrapInNameSpace(*type.struct_def);
        //return "self._tab.get_slot_struct::<&" + lifetime + " " + typname + ">(" + offset_name + ")";
        const std::string default_value = GetDefaultScalarValue(field);
        return "self._tab.get::<" + typname + ">(" + offset_name + ", Some(" + default_value + ")).unwrap()";
      }
      case FullElementType::Struct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        //return "self._tab.get_slot_struct::<" + typname + ">(" + offset_name + ")";
        return "self._tab.get::<&" + lifetime + " " + typname + ">(" + offset_name + ", None)";
      }
      case FullElementType::Table: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        //return "self._tab.get_slot_struct::<" + typname + ">(" + offset_name + ")";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<" + typname + "<" + lifetime + ">>>(" + offset_name + ", None)";
      }
      case FullElementType::UnionValue: {
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<flatbuffers::Table<" + lifetime + ">>>(" + offset_name + ", None)";
        ////const auto typname = WrapInNameSpace(*type.enum_def) + "_UnionEnum";
        //////return "self._tab.get_slot_struct::<" + typname + ">(" + offset_name + ")";
        //////return "self._tab.get_slot_vector::<u8>(" + offset_name + ")";
        //////return "self._tab.get_slot_union_table(" + offset_name + ")";
        ////return "self._tab.get::<" + typname + "<" + lifetime + ">(" + offset_name + ")";
      }
      case FullElementType::UnionKey:
      case FullElementType::EnumKey: {
        const std::string underlying_typname = GenTypeBasic(type, false);
        const std::string typname = WrapInNameSpace(*type.enum_def);
        const std::string default_value = GetDefaultScalarValue(field);
        return "unsafe { ::std::mem::transmute(self._tab.get::<" + underlying_typname + ">(" + offset_name + ", Some(" + default_value + " as " + underlying_typname + ")).unwrap()) }";
      }
      case FullElementType::String: {
        //return "self._tab.get_slot_string(" + offset_name + ").map(|s| s.as_str())";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<&str>>(" + offset_name + ", None)";
      }

      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        //return "self._tab.get_slot_vector::<" + typname + ">(" + offset_name + ")";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<&[" + typname + "]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfBool: {
        //return "self._tab.get_slot_vector::<bool>(" + offset_name + ")";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<&[bool]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfEnumKey: {
        //const auto typname = WrapInNameSpace(*type.VectorType().enum_def);
        const auto typname = WrapInNameSpace(*type.enum_def);
        //const auto typname = GenTypeBasic(type.VectorType(), false);
        //return "self._tab.get_slot_vector::<" + typname + ">(" + offset_name + ")";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<&[" + typname + "]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<&[" + typname + "]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        //return "self._tab.get_slot_vector::<flatbuffers::Offset<" + typname + "<" + lifetime + ">>>(" + offset_name + ")";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<" + typname + "<" + lifetime + ">>>>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfString: {
        //return "self._tab.get_slot_vector::<flatbuffers::Offset<&" + lifetime + " str>>(" + offset_name + ")";
        //return "self._tab.get::<flatbuffers::ForwardsU32Offset<&" + lifetime + " [&" + lifetime + " str]>>(" + offset_name + ", None)";
        return "self._tab.get::<flatbuffers::ForwardsU32Offset<flatbuffers::Vector<flatbuffers::ForwardsU32Offset<&" + lifetime + " str>>>>(" + offset_name + ", None)";
        //return "self._tab.get_slot_vector::<&" + lifetime + " flatbuffers::String<" + lifetime + ">>(" + offset_name + ")";
        //return "self._tab.get_slot_vector::<&" + lifetime + ", &" + lifetime + " flatbuffers::Offset<flatbuffers::String<" + lifetime + ">>>";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def) + "UnionTableOffset";
        return "self._tab.get_slot_vector::<flatbuffers::Offset<" + typname + "<" + lifetime + ">>>(" + offset_name + ")";
        //return "self._tab.get_slot_vector::<&" + lifetime + " " + typname + "<" + lifetime + ">>(" + offset_name + ")";
      }
    }
  }

  bool ElementTypeUsesOption(const Type& type) {
    const auto et = GetElementType(type);

    switch (GetContainerType(type)) {
      case ContainerType::Vector:
      case ContainerType::Union: {
        switch (GetElementType(type)) {
          case ElementType::UnionEnumValue: {
            return false;
          }
          default: {
            return true;
          }
        }
      }
      case ContainerType::Enum: {
        return false;
      }
      case ContainerType::None: {
        switch (et) {
          case ElementType::Struct:
          case ElementType::Table:
          case ElementType::String:
          case ElementType::UnionMember: {
            return true;
          }
          case ElementType::Bool:
          case ElementType::Number:
          case ElementType::EnumValue:
          case ElementType::UnionEnumValue: {
            return false;
          }
        }
      }
    }
  }

  //UNNEEDED void GenParam(const FieldDef &field, bool direct, const char *prefix,
  //UNNEEDED               const std::string &lifetime, const std::string tmpl) {
  //UNNEEDED   code_.SetValue("PRE", prefix);
  //UNNEEDED   code_.SetValue("PARAM_NAME", Name(field));
  //UNNEEDED   //code_.SetValue("PARAM_LIFETIME", lifetime);
  //UNNEEDED   if (direct && field.value.type.base_type == BASE_TYPE_STRING) {
  //UNNEEDED     code_.SetValue("PARAM_TYPE", "Option<&" + lifetime + "str>");
  //UNNEEDED     code_.SetValue("PARAM_VALUE", "nullptr");
  //UNNEEDED   //} else if (IsStruct(field.value.type)) {
  //UNNEEDED   //    code_.SetValue("PARAM_TYPE", GenTypeWire(field.value.type, " ", lifetime, true));
  //UNNEEDED   //    code_.SetValue("PARAM_VALUE", "None");
  //UNNEEDED   } else if (direct && field.value.type.base_type == BASE_TYPE_VECTOR) {
  //UNNEEDED     const auto vtype = field.value.type.VectorType();
  //UNNEEDED     std::string type;
  //UNNEEDED     if (IsStruct(vtype)) {
  //UNNEEDED       type = WrapInNameSpace(*vtype.struct_def);
  //UNNEEDED       //std::string s;
  //UNNEEDED       //s.append("Option<&"); s.append(lifetime); s.append("[&");
  //UNNEEDED       //s.append(lifetime) ; s.append(type); s.append("]>");
  //UNNEEDED       //code_.SetValue("PARAM_TYPE", s);
  //UNNEEDED       code_.SetValue("PARAM_TYPE", "Option<&" + lifetime + "[&" + lifetime + type + "]>");
  //UNNEEDED     } else {
  //UNNEEDED       type = GenTypeWire(vtype, "", lifetime, false);
  //UNNEEDED       code_.SetValue("PARAM_TYPE", "Option<&" + lifetime + "[" + type + "]>");
  //UNNEEDED       //code_.SetValue("PARAM_TYPE", "Option<flatbuffers::LabeledVectorUOffsetT<" + type + ">>");
  //UNNEEDED       //code_.SetValue("PARAM_TYPE", "Option<flatbuffers" + type + "]>");
  //UNNEEDED     }
  //UNNEEDED     code_.SetValue("PARAM_VALUE", "nullptr");
  //UNNEEDED   } else if (IsStruct(field.value.type)) {
  //UNNEEDED     code_.SetValue("PARAM_TYPE", "Option<&" + lifetime + " " + GenTypeWire(field.value.type, " ", lifetime, true) + ">");
  //UNNEEDED     code_.SetValue("PARAM_VALUE", "/* sup */" + GetDefaultScalarValue(field));
  //UNNEEDED   } else if (field.value.type.base_type == BASE_TYPE_UNION) {
  //UNNEEDED     code_.SetValue("PARAM_TYPE", "Option<flatbuffers::LabeledUOffsetT<flatbuffers::UnionOffset>>");
  //UNNEEDED     code_.SetValue("PARAM_VALUE", "None");
  //UNNEEDED   } else {
  //UNNEEDED     code_.SetValue("PARAM_TYPE", GenTypeWire(field.value.type, " ", lifetime, true));
  //UNNEEDED     const std::string type_suffix = GenTypeBasic(field.value.type, true);
  //UNNEEDED     //code_.SetValue("PARAM_VALUE", GetDefaultScalarValue(field) + type_suffix + ".into()");
  //UNNEEDED     //code_.SetValue("PARAM_VALUE", GetDefaultScalarValue(field) + ".into()");
  //UNNEEDED     code_.SetValue("PARAM_VALUE", "/* yo */" + GetDefaultScalarValue(field));
  //UNNEEDED   }
  //UNNEEDED   code_ += tmpl;
  //UNNEEDED   //code_ += "{{PRE}}{{PARAM_NAME}}: {{PARAM_TYPE}} /* = {{PARAM_VALUE}} */\\";
  //UNNEEDED }

  //UNNEEDED // Generate a member, including a default value for scalars and raw pointers.
  //UNNEEDED void GenMember(const FieldDef &field) {
  //UNNEEDED   if (!field.deprecated &&  // Deprecated fields won't be accessible.
  //UNNEEDED       field.value.type.base_type != BASE_TYPE_UTYPE &&
  //UNNEEDED       (field.value.type.base_type != BASE_TYPE_VECTOR ||
  //UNNEEDED        field.value.type.element != BASE_TYPE_UTYPE)) {
  //UNNEEDED     auto type = GenTypeNative(field.value.type, false, field);
  //UNNEEDED     auto cpp_type = field.attributes.Lookup("cpp_type");
  //UNNEEDED     auto full_type = (cpp_type ? cpp_type->constant + " *" : type + " ");
  //UNNEEDED     code_.SetValue("FIELD_TYPE", full_type);
  //UNNEEDED     code_.SetValue("FIELD_NAME", Name(field));
  //UNNEEDED     code_ += "  {{FIELD_TYPE}}{{FIELD_NAME}};";
  //UNNEEDED   }
  //UNNEEDED }

  // Generate the default constructor for this struct. Properly initialize all
  // scalar members with default values.
  void GenDefaultConstructor(const StructDef &struct_def) {
    std::string initializer_list;
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (!field.deprecated &&  // Deprecated fields won't be accessible.
          field.value.type.base_type != BASE_TYPE_UTYPE) {
        auto cpp_type = field.attributes.Lookup("cpp_type");
        // Scalar types get parsed defaults, raw pointers get nullptrs.
        if (IsScalar(field.value.type.base_type)) {
          if (!initializer_list.empty()) { initializer_list += ",\n        "; }
          initializer_list += Name(field);
          initializer_list += "(" + GetDefaultScalarValue(field) + ")";
        } else if (field.value.type.base_type == BASE_TYPE_STRUCT) {
          if (IsStruct(field.value.type)) {
            auto native_default = field.attributes.Lookup("native_default");
            if (native_default) {
              if (!initializer_list.empty()) {
                initializer_list += ",\n        ";
              }
              initializer_list +=
                  Name(field) + "(" + native_default->constant + ")";
            }
          }
        } else if (cpp_type) {
          if (!initializer_list.empty()) { initializer_list += ",\n        "; }
          initializer_list += Name(field) + "(0)";
        }
      }
    }
    if (!initializer_list.empty()) {
      initializer_list = "\n      : " + initializer_list;
    }

    code_.SetValue("NATIVE_NAME",
                   NativeName(Name(struct_def), &struct_def, parser_.opts));
    code_.SetValue("INIT_LIST", initializer_list);

    code_ += "  {{NATIVE_NAME}}(){{INIT_LIST}} {";
    code_ += "  }";
  }

  void GenOperatorNewDelete(const StructDef &struct_def) {
    if (auto native_custom_alloc =
            struct_def.attributes.Lookup("native_custom_alloc")) {
      code_ += "  inline void *operator new (std::size_t count) {";
      code_ += "    return " + native_custom_alloc->constant +
               "<{{NATIVE_NAME}}>().allocate(count / sizeof({{NATIVE_NAME}}));";
      code_ += "  }";
      code_ += "  inline void operator delete (void *ptr) {";
      code_ += "    return " + native_custom_alloc->constant +
               "<{{NATIVE_NAME}}>().deallocate(static_cast<{{NATIVE_NAME}}*>("
               "ptr),1);";
      code_ += "  }";
    }
  }

  //TODO void GenNativeTable(const StructDef &struct_def) {
  //TODO   assert(false);
  //TODO   const auto native_name =
  //TODO       NativeName(Name(struct_def), &struct_def, parser_.opts);
  //TODO   code_.SetValue("STRUCT_NAME", Name(struct_def));
  //TODO   code_.SetValue("NATIVE_NAME", native_name);

  //TODO   // Generate a C++ object that can hold an unpacked version of this table.
  //TODO   code_ += "pub struct {{NATIVE_NAME}} : public flatbuffers::NativeTable {";
  //TODO   code_ += "  typedef {{STRUCT_NAME}} TableType;";
  //TODO   GenFullyQualifiedNameGetter(struct_def, native_name);
  //TODO   for (auto it = struct_def.fields.vec.begin();
  //TODO        it != struct_def.fields.vec.end(); ++it) {
  //TODO     GenMember(**it);
  //TODO   }
  //TODO   GenOperatorNewDelete(struct_def);
  //TODO   GenDefaultConstructor(struct_def);
  //TODO   code_ += "};";
  //TODO   code_ += "";
  //TODO }

  //TODO // Generate the code to call the appropriate Verify function(s) for a field.
  //TODO void GenVerifyCall(const FieldDef &field, const char *prefix) {
  //TODO   code_.SetValue("PRE", prefix);
  //TODO   code_.SetValue("NAME", "self." + Name(field));
  //TODO   code_.SetValue("REQUIRED", field.required ? "_required" : "");
  //TODO   code_.SetValue("SIZE", GenTypeSize(field.value.type));
  //TODO   code_.SetValue("OFFSET", GenFieldOffsetName(field));
  //TODO   if (IsScalar(field.value.type.base_type) || IsStruct(field.value.type)) {
  //TODO     code_ += "{{PRE}}flatbuffers::verify_field{{REQUIRED}}::<{{SIZE}}>"
  //TODO              "(verifier, {{STRUCT_NAME}}::{{OFFSET}})\\";
  //TODO   } else {
  //TODO     code_ += "{{PRE}}flatbuffers::verify_offset{{REQUIRED}}"
  //TODO              "(verifier, {{STRUCT_NAME}}::{{OFFSET}})\\";
  //TODO   }

  //TODO   switch (field.value.type.base_type) {
  //TODO     case BASE_TYPE_UNION: {
  //TODO       code_.SetValue("ENUM_NAME", field.value.type.enum_def->name);
  //TODO       code_.SetValue("SUFFIX", UnionTypeFieldSuffix());
  //TODO       code_ +=
  //TODO           "{{PRE}}Verify{{ENUM_NAME}}(verifier, {{NAME}}(), "
  //TODO           "{{NAME}}{{SUFFIX}}())\\";
  //TODO       break;
  //TODO     }
  //TODO     case BASE_TYPE_STRUCT: {
  //TODO       if (!field.value.type.struct_def->fixed) {
  //TODO         code_ += "{{PRE}}verifier.verify_table({{NAME}}())\\";
  //TODO       }
  //TODO       break;
  //TODO     }
  //TODO     case BASE_TYPE_STRING: {
  //TODO       code_ += "{{PRE}}verifier.verify({{NAME}}())\\";
  //TODO       break;
  //TODO     }
  //TODO     case BASE_TYPE_VECTOR: {
  //TODO       code_ += "{{PRE}}verifier.verify({{NAME}}())\\";

  //TODO       switch (field.value.type.element) {
  //TODO         case BASE_TYPE_STRING: {
  //TODO           code_ += "{{PRE}}verifier.verify_vector_of_strings({{NAME}}())\\";
  //TODO           break;
  //TODO         }
  //TODO         case BASE_TYPE_STRUCT: {
  //TODO           if (!field.value.type.struct_def->fixed) {
  //TODO             code_ += "{{PRE}}verifier.verify_vector_of_tables({{NAME}}())\\";
  //TODO           }
  //TODO           break;
  //TODO         }
  //TODO         case BASE_TYPE_UNION: {
  //TODO           code_.SetValue("ENUM_NAME", field.value.type.enum_def->name);
  //TODO           code_ +=
  //TODO               "{{PRE}}Verify{{ENUM_NAME}}Vector(verifier, {{NAME}}(), "
  //TODO               "{{NAME}}_type())\\";
  //TODO           break;
  //TODO         }
  //TODO         default: break;
  //TODO       }
  //TODO       break;
  //TODO     }
  //TODO     default: { break; }
  //TODO   }
  //TODO }

  // Generate an accessor struct, builder structs & function for a table.
  void GenTable(const StructDef &struct_def) {
    //if (parser_.opts.generate_object_based_api) { GenNativeTable(struct_def); }

    // Generate an accessor struct, with methods of the form:
    // type name() const { return GetField<type>(offset, defaultval); }
    GenComment(struct_def.doc_comment);

    code_.SetValue("STRUCT_NAME", Name(struct_def));
    code_.SetValue("OFFSET_TYPELABEL", Name(struct_def) + "Offset");
    code_ += "pub enum {{OFFSET_TYPELABEL}} {}";
    code_ += "#[derive(Copy, Clone, PartialEq)]";
    code_ += "pub struct {{STRUCT_NAME}}<'a> {";
    code_ += "  pub _tab: flatbuffers::Table<'a>,";
    code_ += "  _phantom: PhantomData<&'a ()>,";
    code_ += "}";
    code_ += "impl<'a> flatbuffers::Follow<'a> for {{STRUCT_NAME}}<'a> {";
    code_ += "    type Inner = {{STRUCT_NAME}}<'a>;";
    code_ += "    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {";
    code_ += "        Self { _tab: flatbuffers::Table { buf: buf, loc: loc }, _phantom: PhantomData }";
    code_ += "    }";
    code_ += "}";
    code_ += "impl<'a> {{STRUCT_NAME}}<'a> /* private flatbuffers::Table */ {";
    code_ += "    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {";
    code_ += "        {{STRUCT_NAME}} {";
    code_ += "            _tab: table,";
    code_ += "            _phantom: PhantomData,";
    code_ += "        }";
    code_ += "    }";

    GenFullyQualifiedNameGetter(struct_def, Name(struct_def));

    // Generate field id constants.
    if (struct_def.fields.vec.size() > 0) {
      //code_.SetValue("SEP", "");
      //code_ += "  enum {";
      for (auto it = struct_def.fields.vec.begin();
           it != struct_def.fields.vec.end(); ++it) {
        const auto &field = **it;
        if (field.deprecated) {
          // Deprecated fields won't be accessible.
          continue;
        }

        code_.SetValue("OFFSET_NAME", GenFieldOffsetName(field));
        code_.SetValue("OFFSET_VALUE", NumToString(field.value.offset));
        code_ += "    pub const {{OFFSET_NAME}}: flatbuffers::VOffsetT = {{OFFSET_VALUE}};";
      }
      code_ += "";
    }

    // Generate the accessors.
    const std::string offset_prefix = Name(struct_def);
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (field.deprecated) {
        // Deprecated fields won't be accessible.
        continue;
      }

      code_.SetValue("FIELD_NAME", Name(field));
      code_.SetValue("RETURN_TYPE", GenTableAccessorFuncReturnType(field, "'a"));
      code_.SetValue("FUNC_BODY", GenTableAccessorFuncBody(field, "'a", offset_prefix));

      GenComment(field.doc_comment, "  ");
      code_ += "  #[inline]";
      code_ += "  pub fn {{FIELD_NAME}}(&'a self) -> {{RETURN_TYPE}} {";
      code_ += "    {{FUNC_BODY}}";
      code_ += "  }";


      auto nested = field.attributes.Lookup("nested_flatbuffer");
      if (nested) {
        std::string qualified_name = nested->constant;
        auto nested_root = parser_.LookupStruct(nested->constant);
        if (nested_root == nullptr) {
          qualified_name = parser_.current_namespace_->GetFullyQualifiedName(
              nested->constant);
          nested_root = parser_.LookupStruct(qualified_name);
        }
        FLATBUFFERS_ASSERT(nested_root);  // Guaranteed to exist by parser.
        (void)nested_root;

        code_.SetValue("OFFSET_NAME", offset_prefix + "::" + GenFieldOffsetName(field));
        code_ += "  pub fn {{FIELD_NAME}}_nested_flatbuffer(&'a self) -> Option<{{STRUCT_NAME}}<'a>> {";
        code_ += "     match self.{{FIELD_NAME}}() {";
        code_ += "         None => { None }";
        code_ += "         Some(data) => {";
        code_ += "             use self::flatbuffers::Follow;";
        code_ += "             Some(<flatbuffers::ForwardsU32Offset<{{STRUCT_NAME}}<'a>>>::follow(data, 0))";
        code_ += "         },";
        code_ += "     }";
        code_ += "  }";
      }

      // Generate a comparison function for this field if it is a key.
      if (field.key) {
        std::cerr << "field with comparison key skipped because it is unsupported in rust" << std::endl;
      }
    }

    code_ += "}";  // End of table.
    code_ += "";

    // Explicit specializations for union accessors
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (field.deprecated || field.value.type.base_type != BASE_TYPE_UNION) {
        continue;
      }

      auto u = field.value.type.enum_def;
      if (u->uses_type_aliases) continue;

      code_.SetValue("FIELD_NAME", Name(field));

      for (auto u_it = u->vals.vec.begin(); u_it != u->vals.vec.end(); ++u_it) {
        auto &ev = **u_it;
        if (ev.union_type.base_type == BASE_TYPE_NONE) { continue; }

        auto full_struct_name = GetUnionElement(ev, true, true);

        code_.SetValue(
            "U_ELEMENT_TYPE",
            WrapInNameSpace(u->defined_namespace, GetEnumValUse(*u, ev)));
        code_.SetValue("U_FIELD_TYPE", "&" + full_struct_name + "");
        code_.SetValue("U_ELEMENT_NAME", full_struct_name);
        code_.SetValue("U_FIELD_NAME", Name(field) + "_as_" + Name(ev));

        // `template<> const T *union_name_as<T>() const` accessor.
        code_ += "//TODO: inject these functions into impl for type";
        code_ += "//#[inline]";
        code_ +=
            "//fn {{STRUCT_NAME}}_MEMBER_{{FIELD_NAME}}_as"
            "_X_{{U_ELEMENT_NAME}}_X() -> {{U_FIELD_TYPE}} {";
        code_ += "//  return {{U_FIELD_NAME}}();";
        code_ += "//}";
        code_ += "//";
      }
    }

    GenBuilders(struct_def);
  }

  void GenBuilders(const StructDef &struct_def) {
    code_.SetValue("STRUCT_NAME", Name(struct_def));
    code_.SetValue("OFFSET_TYPELABEL", Name(struct_def) + "Offset");
    code_.SetValue("PARENT_LIFETIME",
        StructNeedsLifetime(struct_def) ? "<'a>" : "");

    // Generate an args struct:
    code_ += "pub struct {{STRUCT_NAME}}Args<'a> {";
    //code_ += "  fbb_: &'a mut flatbuffers::FlatBufferBuilder,";
    //code_ += "  start_: flatbuffers::UOffsetT,";
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (!field.deprecated) {
        // TODO: required-ness
        code_.SetValue("PARAM_NAME", Name(field));
        code_.SetValue("PARAM_TYPE", GenBuilderArgsDefnType(field, "'a "));
        code_ += "    pub {{PARAM_NAME}}: {{PARAM_TYPE}},";
      }
    }
    code_ += "    pub _phantom: PhantomData<&'a ()>, // pub for default trait";
    code_ += "}";

    // Generate an impl of Default for the *Args type:
    code_ += "impl<'a> Default for {{STRUCT_NAME}}Args<'a> {";
    code_ += "    fn default() -> Self {";
    code_ += "        {{STRUCT_NAME}}Args {";
    for (auto it = struct_def.fields.vec.begin();
        it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (!field.deprecated) {
        code_.SetValue("PARAM_VALUE", GenBuilderArgsDefaultValue(field));
        //code_.SetValue("PARAM_VALUE", "None");
        if (field.required) {
          code_ += " // required";
        }
        code_.SetValue("PARAM_NAME", Name(field));
        code_ += "            {{PARAM_NAME}}: {{PARAM_VALUE}},";
        //GenParam(field, false, "            ", "", tmpl);
      }
    }
    code_ += "            _phantom: PhantomData,";
    code_ += "        }";
    code_ += "    }";
    code_ += "}";

    // Generate a builder struct:
    code_ += "pub struct {{STRUCT_NAME}}Builder<'a: 'b, 'b> {";
    code_ += "  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,";
    code_ += "  start_: flatbuffers::Offset<flatbuffers::TableOffset>,";
    code_ += "}";

    // Generate builder functions:
    //code_ += "impl{{PARENT_LIFETIME}} {{STRUCT_NAME}}Builder{{PARENT_LIFETIME}} {";
    code_ += "impl<'a: 'b, 'b> {{STRUCT_NAME}}Builder<'a, 'b> {";
    bool has_string_or_vector_fields = false;
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (!field.deprecated) {
        const bool is_scalar = IsScalar(field.value.type.base_type);
        const bool is_string = field.value.type.base_type == BASE_TYPE_STRING;
        const bool is_vector = field.value.type.base_type == BASE_TYPE_VECTOR;
        if (is_string || is_vector) { has_string_or_vector_fields = true; }

        std::string offset = GenFieldOffsetName(field);
        std::string name = GenUnderlyingCast(field, false, Name(field));
        std::string value = is_scalar ? GenDefaultConstant(field) : "";

        // Generate accessor functions of the form:
        // fn add_name(type name) {
        //   fbb_.AddElement::<type>(offset, name, default);
        // }
        code_.SetValue("FIELD_NAME", Name(field));
        code_.SetValue("FIELD_CAST", GenBuilderArgsAddFuncFieldCast(field));
        code_.SetValue("FIELD_OFFSET", Name(struct_def) + "::" + offset);
        code_.SetValue("FIELD_TYPE", GenBuilderArgsAddFuncType(field, "'b "));
        code_.SetValue("FUNC_BODY", GenBuilderArgsAddFuncBody(field));
        code_ += "  pub fn add_{{FIELD_NAME}}(&mut self, {{FIELD_NAME}}: {{FIELD_TYPE}}) {";
        if (is_scalar) {
          code_.SetValue("FIELD_DEFAULT_VALUE", GenBuilderAddFuncDefaultValue(field));
          code_ += "    {{FUNC_BODY}}({{FIELD_OFFSET}}, {{FIELD_NAME}}{{FIELD_CAST}}, {{FIELD_DEFAULT_VALUE}});";
        } else {
          code_ += "    {{FUNC_BODY}}({{FIELD_OFFSET}}, {{FIELD_NAME}}{{FIELD_CAST}});";
        }
        code_ += "  }";
        //XXcode_ += "  }";
        //XXcode_ += "  pub fn add_{{FIELD_NAME}}(&mut self, {{FIELD_NAME}}: {{FIELD_TYPE}}) {";
        //XXcode_ += "    {{ADD_FN}}(\\";
        //XXif (is_scalar) {
        //XX  code_ += "{{ADD_OFFSET}}, {{ADD_NAME}}, {{ADD_VALUE}});";
        //XX} else {
        //XX  code_ += "{{ADD_OFFSET}}, {{ADD_NAME}});";
        //XX}
        //XXcode_ += "  }";
        //XXif (IsStruct(field.value.type)) {
        //XX  code_.SetValue("FIELD_TYPE", "&" + GenTypeWire(field.value.type, " ", "", true));
        //XX} else {
        //XX  code_.SetValue("FIELD_TYPE", GenTypeWire(field.value.type, " ", "", true));
        //XX}
        //XXcode_.SetValue("ADD_OFFSET", Name(struct_def) + "::" + offset);
        //XXcode_.SetValue("ADD_NAME", name);
        //XXif (is_scalar) {
        //XX  const auto type = GenTypeWire(field.value.type, "", "", false);
        //XX  code_.SetValue("ADD_VALUE", value);
        //XX  code_.SetValue("ADD_FN", "self.fbb_.push_slot_scalar::<" + type + ">");
        //XX} else if (IsStruct(field.value.type)) {
        //XX  const auto type = GenTypeWire(field.value.type, "", "", false);
        //XX  code_.SetValue("ADD_VALUE", value);
        //XX  code_.SetValue("ADD_FN", "self.fbb_.push_slot_struct::<" + type + ">");
        //XX} else if (field.value.type.base_type == BASE_TYPE_UNION) {
        //XX  //code_.SetValue("ADD_FN", "push_slot_scalar::<flatbuffers::LabeledUOffsetT<" + type + ">>");
        //XX  code_.SetValue("ADD_VALUE", value);
        //XX  //code_.SetValue("ADD_FN", "self.fbb_.push_slot_labeled_uoffset_relative_from_option");
        //XX  code_.SetValue("ADD_FN", "self.fbb_.push_slot_labeled_uoffset_relative");
        //XX} else {
        //XX  code_.SetValue("ADD_VALUE", value);
        //XX  code_.SetValue("ADD_FN", "self.fbb_.push_slot_labeled_uoffset_relative");
        //XX}

        //XXcode_ += "  pub fn add_{{FIELD_NAME}}(&mut self, {{FIELD_NAME}}: {{FIELD_TYPE}}) {";
        //XXcode_ += "    {{ADD_FN}}(\\";
        //XXif (is_scalar) {
        //XX  code_ += "{{ADD_OFFSET}}, {{ADD_NAME}}, {{ADD_VALUE}});";
        //XX} else {
        //XX  code_ += "{{ADD_OFFSET}}, {{ADD_NAME}});";
        //XX}
        //XXcode_ += "  }";
      }
    }

    // Builder constructor
    code_ +=
        "  pub fn new"
        "(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> "
        "{{STRUCT_NAME}}Builder<'a, 'b> {";
    code_.SetValue("NUM_FIELDS", NumToString(struct_def.fields.vec.size()));
    code_ += "    let start = _fbb.start_table({{NUM_FIELDS}});";
    code_ += "    {{STRUCT_NAME}}Builder {";
    code_ += "      fbb_: _fbb,";
    code_ += "      start_: start,";
    code_ += "    }";
    code_ += "  }";

    // Assignment operator;
    code_ +=
        "  // {{STRUCT_NAME}}Builder &operator="
        "(const {{STRUCT_NAME}}Builder &);";

    // Finish() function.
    code_ += "  //pub fn finish<'c>(mut self) -> flatbuffers::Offset<flatbuffers::TableOffset> {";
    code_ += "  pub fn finish<'c>(mut self) -> flatbuffers::Offset<{{STRUCT_NAME}}<'a>> {";
    code_ += "    let o = self.fbb_.end_table(self.start_);";
    code_ += "    //let o = flatbuffers::Offset::<{{STRUCT_NAME}}<'a>>::new(end);";

    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (!field.deprecated && field.required) {
        code_.SetValue("FIELD_NAME", Name(field));
        code_.SetValue("OFFSET_NAME", GenFieldOffsetName(field));
        code_ += "    self.fbb_.required(&o, {{STRUCT_NAME}}::{{OFFSET_NAME}});";
      }
    }
    code_ += "    flatbuffers::Offset::new(o.value())";
    code_ += "  }";
    code_ += "}";
    code_ += "";

    // Generate a convenient CreateX function that uses the above builder
    // to create a table in one go.
    code_ += "#[inline]";
    code_ += "pub fn Create{{STRUCT_NAME}}<'a: 'b, 'b: 'c, 'c>(";
    code_ += "    _fbb: &'c mut flatbuffers::FlatBufferBuilder<'a>,";
    code_ += "    args: &'b {{STRUCT_NAME}}Args<'b>) -> \\";
    code_ += "flatbuffers::Offset<{{STRUCT_NAME}}<'a>> {";
    //for (auto it = struct_def.fields.vec.begin();
    //     it != struct_def.fields.vec.end(); ++it) {
    //  const auto &field = **it;
    //  if (!field.deprecated) { GenParam(field, false, ",\n    "); }
    //}

    code_ += "  let mut builder = {{STRUCT_NAME}}Builder::new(_fbb);";
    for (size_t size = struct_def.sortbysize ? sizeof(largest_scalar_t) : 1;
         size; size /= 2) {
      for (auto it = struct_def.fields.vec.rbegin();
           it != struct_def.fields.vec.rend(); ++it) {
        const auto &field = **it;
        if (!field.deprecated && (!struct_def.sortbysize ||
                                  size == SizeOf(field.value.type.base_type))) {
          code_.SetValue("FIELD_NAME", Name(field));
          if (ElementTypeUsesOption(field.value.type)) {
            code_ += "  if let Some(x) = args.{{FIELD_NAME}} { builder.add_{{FIELD_NAME}}(x); }";
          } else {
            code_ += "  builder.add_{{FIELD_NAME}}(args.{{FIELD_NAME}});";
          }
        }
      }
    }
    code_ += "  builder.finish()";
    code_ += "}";
    code_ += "";

  //TODO  // Generate a CreateXDirect function with vector types as parameters
  //TODO  // TODO
  //TODO  if (has_string_or_vector_fields && false) {
  //TODO    code_ += "#[inline]";
  //TODO    code_ += "pub fn Create{{STRUCT_NAME}}Direct<'fbb>(";
  //TODO    code_ += "    _fbb: &'fbb mut flatbuffers::FlatBufferBuilder<'fbb>\\";
  //TODO    for (auto it = struct_def.fields.vec.begin();
  //TODO         it != struct_def.fields.vec.end(); ++it) {
  //TODO      const auto &field = **it;
  //TODO      if (!field.deprecated) {
  //TODO        GenParam(field, true, ",\n    ", "",
  //TODO                "{{PRE}}{{PARAM_NAME}}: {{PARAM_TYPE}} /* = {{PARAM_VALUE}} */\\");
  //TODO      }
  //TODO    }

  //TODO    // Need to call "Create" with the struct namespace.
  //TODO    const auto qualified_create_name =
  //TODO        struct_def.defined_namespace->GetFullyQualifiedName("Create");
  //TODO    code_.SetValue("CREATE_NAME", TranslateNameSpace(qualified_create_name));

  //TODO    code_ += ") -> flatbuffers::LabeledUOffsetT<{{STRUCT_NAME}}<'fbb>> {";
  //TODO    for (auto it = struct_def.fields.vec.begin();
  //TODO         it != struct_def.fields.vec.end(); ++it) {
  //TODO      const auto &field = **it;
  //TODO      if (!field.deprecated) {
  //TODO        code_.SetValue("FIELD_NAME", Name(field));

  //TODO        if (field.value.type.base_type == BASE_TYPE_STRING) {
  //TODO          code_ += "  let _offset_{{FIELD_NAME}} = if let Some(x) = {{FIELD_NAME}} { _fbb.create_string(x) } else { flatbuffers::LabeledUOffsetT::new(0) };";
  //TODO        } else if (field.value.type.base_type == BASE_TYPE_VECTOR) {
  //TODO          const auto vtype = field.value.type.VectorType();
  //TODO          if (IsStruct(vtype)) {
  //TODO            const auto type = WrapInNameSpace(*vtype.struct_def);
  //TODO            code_ += "  let _offset_{{FIELD_NAME}} = if let Some(x) = {{FIELD_NAME}} { _fbb.create_vector_of_structs::<&" + type + ">(x /* slice */) } else { flatbuffers::LabeledUOffsetT::new(0) };";
  //TODO          } else {
  //TODO            const auto type = GenTypeWire(vtype, "", "", false);
  //TODO            code_ += "  let _offset_{{FIELD_NAME}} = if let Some(x) = {{FIELD_NAME}} { _fbb.create_vector::<" + type + ">(x /* slice */) } else { flatbuffers::LabeledUOffsetT::new(0) };";
  //TODO          }
  //TODO        } else {
  //TODO          // PASS
  //TODO        }
  //TODO      }
  //TODO    }
  //TODO    code_ += "  return Create{{STRUCT_NAME}}(";
  //TODO    code_ += "      _fbb\\";
  //TODO    for (auto it = struct_def.fields.vec.begin();
  //TODO         it != struct_def.fields.vec.end(); ++it) {
  //TODO      const auto &field = **it;
  //TODO      if (!field.deprecated) {
  //TODO        code_.SetValue("FIELD_NAME", Name(field));

  //TODO        if (field.value.type.base_type == BASE_TYPE_STRING) {
  //TODO          code_ += ",\n      _offset_{{FIELD_NAME}}\\";
  //TODO        } else if (field.value.type.base_type == BASE_TYPE_VECTOR) {
  //TODO          code_ += ",\n      _offset_{{FIELD_NAME}}\\";
  //TODO        } else {
  //TODO          code_ += ",\n      {{FIELD_NAME}}\\";
  //TODO        }
  //TODO      }
  //TODO    }
  //TODO    code_ += ");";
  //TODO    code_ += "}";
  //TODO    code_ += "";
  //TODO  }
  }

  std::string GenUnionUnpackVal(const FieldDef &afield,
                                const char *vec_elem_access,
                                const char *vec_type_access) {
    return afield.value.type.enum_def->name + "Union::UnPack(" + "_e" +
           vec_elem_access + ", " + Name(afield) + UnionTypeFieldSuffix() +
           "()" + vec_type_access + ", _resolver)";
  }

  std::string GenUnpackVal(const Type &type, const std::string &val,
                           bool invector, const FieldDef &afield) {
    switch (type.base_type) {
      case BASE_TYPE_STRING: {
        return val + "->str()";
      }
      case BASE_TYPE_STRUCT: {
        const auto name = WrapInNameSpace(*type.struct_def);
        if (IsStruct(type)) {
          auto native_type = type.struct_def->attributes.Lookup("native_type");
          if (native_type) {
            return "flatbuffers::UnPack(*" + val + ")";
          } else if (invector || afield.native_inline) {
            return "*" + val;
          } else {
            const auto ptype = GenTypeNativePtr(name, &afield, true);
            return ptype + "(new " + name + "(*" + val + "))";
          }
        } else {
          const auto ptype = GenTypeNativePtr(
              NativeName(name, type.struct_def, parser_.opts), &afield, true);
          return ptype + "(" + val + "->UnPack(_resolver))";
        }
      }
      case BASE_TYPE_UNION: {
        return GenUnionUnpackVal(
            afield, invector ? "->Get(_i)" : "",
            invector ? ("->GetEnum<" + type.enum_def->name + ">(_i)").c_str()
                     : "");
      }
      default: {
        return val;
        break;
      }
    }
  };

  std::string GenUnpackFieldStatement(const FieldDef &field,
                                      const FieldDef *union_field) {
    std::string code;
    switch (field.value.type.base_type) {
      case BASE_TYPE_VECTOR: {
        std::string indexing;
        if (field.value.type.enum_def) {
          indexing += "(" + field.value.type.enum_def->name + ")";
        }
        indexing += "_e->Get(_i)";
        if (field.value.type.element == BASE_TYPE_BOOL) { indexing += " != 0"; }

        // Generate code that pushes data from _e to _o in the form:
        //   for (UOffsetT i = 0; i < _e->size(); ++i) {
        //     _o->field.push_back(_e->Get(_i));
        //   }
        auto name = Name(field);
        if (field.value.type.element == BASE_TYPE_UTYPE) {
          name = StripUnionType(Name(field));
        }
        auto access =
            field.value.type.element == BASE_TYPE_UTYPE
                ? ".type"
                : (field.value.type.element == BASE_TYPE_UNION ? ".value" : "");
        code += "{ _o->" + name + ".resize(_e->size()); ";
        code += "for (flatbuffers::UOffsetT _i = 0;";
        code += " _i < _e->size(); _i++) { ";
        code += "_o->" + name + "[_i]" + access + " = ";
        code +=
            GenUnpackVal(field.value.type.VectorType(), indexing, true, field);
        code += "; } }";
        break;
      }
      case BASE_TYPE_UTYPE: {
        assert(union_field->value.type.base_type == BASE_TYPE_UNION);
        // Generate code that sets the union type, of the form:
        //   _o->field.type = _e;
        code += "_o->" + union_field->name + ".type = _e;";
        break;
      }
      case BASE_TYPE_UNION: {
        // Generate code that sets the union value, of the form:
        //   _o->field.value = Union::Unpack(_e, field_type(), resolver);
        code += "_o->" + Name(field) + ".value = ";
        code += GenUnionUnpackVal(field, "", "");
        code += ";";
        break;
      }
      default: {
        auto cpp_type = field.attributes.Lookup("cpp_type");
        if (cpp_type) {
          // Generate code that resolves the cpp pointer type, of the form:
          //  if (resolver)
          //    (*resolver)(&_o->field, (hash_value_t)(_e));
          //  else
          //    _o->field = nullptr;
          code += "if (_resolver) ";
          code += "(*_resolver)";
          code += "(reinterpret_cast<void **>(&_o->" + Name(field) + "), ";
          code += "static_cast<flatbuffers::hash_value_t>(_e));";
          code += " else ";
          code += "_o->" + Name(field) + " = nullptr;";
        } else {
          // Generate code for assigning the value, of the form:
          //  _o->field = value;
          code += "_o->" + Name(field) + " = ";
          code += GenUnpackVal(field.value.type, "_e", false, field) + ";";
        }
        break;
      }
    }
    return code;
  }

  std::string GenCreateParam(const FieldDef &field) {
    std::string value = "_o->";
    if (field.value.type.base_type == BASE_TYPE_UTYPE) {
      value += StripUnionType(Name(field));
      value += ".type";
    } else {
      value += Name(field);
    }
    if (field.attributes.Lookup("cpp_type")) {
      auto type = GenTypeBasic(field.value.type, false);
      value =
          "_rehasher ? "
          "static_cast<" +
          type + ">((*_rehasher)(" + value + ")) : 0";
    }

    std::string code;
    switch (field.value.type.base_type) {
      // String fields are of the form:
      //   _fbb.create_string(_o->field)
      case BASE_TYPE_STRING: {
        code += "_fbb.create_string(" + value + ")";

        // For optional fields, check to see if there actually is any data
        // in _o->field before attempting to access it.
        if (!field.required) { code = value + ".empty() ? 0 : " + code; }
        break;
      }
      // Vector fields come in several flavours, of the forms:
      //   _fbb.CreateVector(_o->field);
      //   _fbb.CreateVector((const utype*)_o->field.data(), _o->field.size());
      //   _fbb.CreateVectorOfStrings(_o->field)
      //   _fbb.CreateVectorOfStructs(_o->field)
      //   _fbb.CreateVector<Offset<T>>(_o->field.size() [&](size_t i) {
      //     return CreateT(_fbb, _o->Get(i), rehasher);
      //   });
      case BASE_TYPE_VECTOR: {
        auto vector_type = field.value.type.VectorType();
        switch (vector_type.base_type) {
          case BASE_TYPE_STRING: {
            code += "_fbb.create_vector_of_strings(" + value + ")";
            break;
          }
          case BASE_TYPE_STRUCT: {
            if (IsStruct(vector_type)) {
              auto native_type =
                  field.value.type.struct_def->attributes.Lookup("native_type");
              if (native_type) {
                code += "_fbb.CreateVectorOfNativeStructs<";
                code += WrapInNameSpace(*vector_type.struct_def) + ">";
              } else {
                code += "_fbb.CreateVectorOfStructs";
              }
              code += "(" + value + ")";
            } else {
              code += "_fbb.create_vector<flatbuffers::Offset<";
              code += WrapInNameSpace(*vector_type.struct_def) + ">> ";
              code += "(" + value + ".size(), ";
              code += "[](size_t i, _VectorArgs *__va) { ";
              code += "return Create" + vector_type.struct_def->name;
              code += "(*__va->__fbb, __va->_" + value + "[i]" +
                      GenPtrGet(field) + ", ";
              code += "__va->__rehasher); }, &_va )";
            }
            break;
          }
          case BASE_TYPE_BOOL: {
            code += "_fbb.create_vector(" + value + ")";
            break;
          }
          case BASE_TYPE_UNION: {
            code +=
                "_fbb.create_vector<flatbuffers::"
                "Offset<flatbuffers::Void>>(" +
                value +
                ".size(), [](size_t i, _VectorArgs *__va) { "
                "return __va->_" +
                value + "[i].Pack(*__va->__fbb, __va->__rehasher); }, &_va)";
            break;
          }
          case BASE_TYPE_UTYPE: {
            value = StripUnionType(value);
            code += "_fbb.create_vector<u8>(" + value +
                    ".size(), [](size_t i, _VectorArgs *__va) { "
                    "return static_cast<u8>(__va->_" +
                    value + "[i].type); }, &_va)";
            break;
          }
          default: {
            if (field.value.type.enum_def) {
              // For enumerations, we need to get access to the array data for
              // the underlying storage type (eg. uint8_t).
              const auto basetype = GenTypeBasic(
                  field.value.type.enum_def->underlying_type, false);
              code += "_fbb.create_vector((const " + basetype + "*)" + value +
                      ".data(), " + value + ".size())";
            } else {
              code += "_fbb.create_vector(" + value + ")";
            }
            break;
          }
        }

        // For optional fields, check to see if there actually is any data
        // in _o->field before attempting to access it.
        if (!field.required) { code = value + ".size() ? " + code + " : 0"; }
        break;
      }
      case BASE_TYPE_UNION: {
        // _o->field.Pack(_fbb);
        code += value + ".Pack(_fbb)";
        break;
      }
      case BASE_TYPE_STRUCT: {
        if (IsStruct(field.value.type)) {
          auto native_type =
              field.value.type.struct_def->attributes.Lookup("native_type");
          if (native_type) {
            code += "flatbuffers::Pack(" + value + ")";
          } else if (field.native_inline) {
            code += "&" + value;
          } else {
            code += value + " ? " + value + GenPtrGet(field) + " : 0";
          }
        } else {
          // _o->field ? CreateT(_fbb, _o->field.get(), _rehasher);
          const auto type = field.value.type.struct_def->name;
          code += value + " ? Create" + type;
          code += "(_fbb, " + value + GenPtrGet(field) + ", _rehasher)";
          code += " : 0";
        }
        break;
      }
      default: {
        code += value;
        break;
      }
    }
    return code;
  }

  //TODO // Generate code for tables that needs to come after the regular definition.
  //TODO void GenTablePost(const StructDef &struct_def) {
  //TODO   code_.SetValue("STRUCT_NAME", Name(struct_def));
  //TODO   code_.SetValue("NATIVE_NAME",
  //TODO                  NativeName(Name(struct_def), &struct_def, parser_.opts));

  //TODO   //if (parser_.opts.generate_object_based_api) {
  //TODO   //  // Generate the X::UnPack() method.
  //TODO   //  code_ += "inline " +
  //TODO   //           TableUnPackSignature(struct_def, false, parser_.opts) + " {";
  //TODO   //  code_ += "  auto _o = new {{NATIVE_NAME}}();";
  //TODO   //  code_ += "  UnPackTo(_o, _resolver);";
  //TODO   //  code_ += "  return _o;";
  //TODO   //  code_ += "}";
  //TODO   //  code_ += "";

  //TODO   //  code_ += "inline " +
  //TODO   //           TableUnPackToSignature(struct_def, false, parser_.opts) + " {";
  //TODO   //  code_ += "  (void)_o;";
  //TODO   //  code_ += "  (void)_resolver;";

  //TODO   //  for (auto it = struct_def.fields.vec.begin();
  //TODO   //       it != struct_def.fields.vec.end(); ++it) {
  //TODO   //    const auto &field = **it;
  //TODO   //    if (field.deprecated) { continue; }

  //TODO   //    // Assign a value from |this| to |_o|.   Values from |this| are stored
  //TODO   //    // in a variable |_e| by calling this->field_type().  The value is then
  //TODO   //    // assigned to |_o| using the GenUnpackFieldStatement.
  //TODO   //    const bool is_union = field.value.type.base_type == BASE_TYPE_UTYPE;
  //TODO   //    const auto statement =
  //TODO   //        GenUnpackFieldStatement(field, is_union ? *(it + 1) : nullptr);

  //TODO   //    code_.SetValue("FIELD_NAME", Name(field));
  //TODO   //    auto prefix = "  { auto _e = {{FIELD_NAME}}(); ";
  //TODO   //    auto check = IsScalar(field.value.type.base_type) ? "" : "if (_e) ";
  //TODO   //    auto postfix = " };";
  //TODO   //    code_ += std::string(prefix) + check + statement + postfix;
  //TODO   //  }
  //TODO   //  code_ += "}";
  //TODO   //  code_ += "";

  //TODO   //  // Generate the X::Pack member function that simply calls the global
  //TODO   //  // CreateX function.
  //TODO   //  code_ += "inline " + TablePackSignature(struct_def, false, parser_.opts) +
  //TODO   //           " {";
  //TODO   //  code_ += "  return Create{{STRUCT_NAME}}(_fbb, _o, _rehasher);";
  //TODO   //  code_ += "}";
  //TODO   //  code_ += "";

  //TODO   //  // Generate a CreateX method that works with an unpacked C++ object.
  //TODO   //  code_ += "inline " +
  //TODO   //           TableCreateSignature(struct_def, false, parser_.opts) + " {";
  //TODO   //  code_ += "  (void)_rehasher;";
  //TODO   //  code_ += "  (void)_o;";

  //TODO   //  code_ +=
  //TODO   //      "  struct _VectorArgs "
  //TODO   //      "{ flatbuffers::FlatBufferBuilder *__fbb; "
  //TODO   //      "const " +
  //TODO   //      NativeName(Name(struct_def), &struct_def, parser_.opts) +
  //TODO   //      "* __o; "
  //TODO   //      "const flatbuffers::rehasher_function_t *__rehasher; } _va = { "
  //TODO   //      "&_fbb, _o, _rehasher}; (void)_va;";

  //TODO   //  for (auto it = struct_def.fields.vec.begin();
  //TODO   //       it != struct_def.fields.vec.end(); ++it) {
  //TODO   //    auto &field = **it;
  //TODO   //    if (field.deprecated) { continue; }
  //TODO   //    code_ += "  auto _" + Name(field) + " = " + GenCreateParam(field) + ";";
  //TODO   //  }
  //TODO   //  // Need to call "Create" with the struct namespace.
  //TODO   //  const auto qualified_create_name =
  //TODO   //      struct_def.defined_namespace->GetFullyQualifiedName("Create");
  //TODO   //  code_.SetValue("CREATE_NAME", TranslateNameSpace(qualified_create_name));

  //TODO   //  code_ += "  return {{CREATE_NAME}}{{STRUCT_NAME}}(";
  //TODO   //  code_ += "      _fbb\\";
  //TODO   //  for (auto it = struct_def.fields.vec.begin();
  //TODO   //       it != struct_def.fields.vec.end(); ++it) {
  //TODO   //    auto &field = **it;
  //TODO   //    if (field.deprecated) { continue; }

  //TODO   //    bool pass_by_address = false;
  //TODO   //    if (field.value.type.base_type == BASE_TYPE_STRUCT) {
  //TODO   //      if (IsStruct(field.value.type)) {
  //TODO   //        auto native_type =
  //TODO   //            field.value.type.struct_def->attributes.Lookup("native_type");
  //TODO   //        if (native_type) { pass_by_address = true; }
  //TODO   //      }
  //TODO   //    }

  //TODO   //    // Call the CreateX function using values from |_o|.
  //TODO   //    if (pass_by_address) {
  //TODO   //      code_ += ",\n      &_" + Name(field) + "\\";
  //TODO   //    } else {
  //TODO   //      code_ += ",\n      _" + Name(field) + "\\";
  //TODO   //    }
  //TODO   //  }
  //TODO   //  code_ += ");";
  //TODO   //  code_ += "}";
  //TODO   //  code_ += "";
  //TODO   //}
  //TODO }

  static void GenPadding(
      const FieldDef &field, std::string *code_ptr, int *id,
      const std::function<void(int bits, std::string *code_ptr, int *id)> &f) {
    if (field.padding) {
      for (int i = 0; i < 4; i++) {
        if (static_cast<int>(field.padding) & (1 << i)) {
          f((1 << i) * 8, code_ptr, id);
        }
      }
      assert(!(field.padding & ~0xF));
    }
  }

  static void PaddingDefinition(int bits, std::string *code_ptr, int *id) {
    *code_ptr += "  padding" + NumToString((*id)++) + "__: u" + \
                 NumToString(bits) + ",";
  }

  static void PaddingInitializer(int bits, std::string *code_ptr, int *id) {
    (void)bits;
    *code_ptr += "\n        padding" + NumToString((*id)++) + "__: 0,";
  }

  static void PaddingNoop(int bits, std::string *code_ptr, int *id) {
    (void)bits;
    *code_ptr += "    (void)padding" + NumToString((*id)++) + "__,";
  }

  // Generate an accessor struct with constructor for a flatbuffers struct.
  void GenStruct(const StructDef &struct_def) {
    // Generate an accessor struct, with private variables of the form:
    // type name_;
    // Generates manual padding and alignment.
    // Variables are private because they contain little endian data on all
    // platforms.
    GenComment(struct_def.doc_comment);
    code_.SetValue("ALIGN", NumToString(struct_def.minalign));
    code_.SetValue("STRUCT_NAME", Name(struct_def));

    code_ += "// MANUALLY_ALIGNED_STRUCT({{ALIGN}})";
    code_ += "#[repr(C, packed)]";
    code_ += "#[derive(Clone, Copy, Default, Debug, PartialEq)]";

    // TODO: maybe only use lifetimes when needed by members, and skip
    //       PhantomData? use TypeNeedsLifetime.
		code_ += "pub struct {{STRUCT_NAME}} {";

    int padding_id = 0;
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      const bool needs_lifetime = TypeNeedsLifetime(field.value.type);
      const auto lifetime = needs_lifetime ? "<'a>" : "";
      code_.SetValue("FIELD_TYPE",
                     GenTypeGet(field.value.type, "", "", lifetime,
                                false));
      code_.SetValue("FIELD_NAME", Name(field));
      code_ += "  {{FIELD_NAME}}_: {{FIELD_TYPE}},";

      if (field.padding) {
        std::string padding;
        GenPadding(field, &padding, &padding_id, PaddingDefinition);
        code_ += padding;
      }
    }

    code_ += "} // pub struct {{STRUCT_NAME}}";

    // Impl the dummy GeneratedStruct trait to help users write structs
    // correctly:
		code_ += "//impl flatbuffers::GeneratedStruct for {{STRUCT_NAME}} {}";

    // Generate GetFullyQualifiedName
    code_ += "";
		code_ += "impl {{STRUCT_NAME}} {";
    GenFullyQualifiedNameGetter(struct_def, Name(struct_def));

    // Generate a default constructor.
    code_ += "  pub fn Reset(&mut self) {";
    code_ += "    //memset(this, 0, size_of({{STRUCT_NAME}}));";
    code_ += "  }";

    // Generate a constructor that takes all fields as arguments.
    std::string arg_list;
    std::string init_list;
    padding_id = 0;
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      const auto member_name = Name(field) + "_";
      const auto arg_name = "_" + Name(field);
      const auto arg_type =
          GenTypeGet(field.value.type, "", "", "", true);

      if (it != struct_def.fields.vec.begin()) {
        arg_list += ", ";
        //init_list += ";\n      ";
      }
      arg_list += arg_name + ": ";
      arg_list += arg_type;
      init_list += "      " + member_name;
      if (IsScalar(field.value.type.base_type) &&
          !IsFloat(field.value.type.base_type)) {
        auto type = GenUnderlyingCast(field, false, arg_name);
        init_list += ": flatbuffers::endian_scalar(" + type + "),\n";
      } else {
        init_list += ": " + arg_name + ",\n";
      }
      //if (field.padding) {
      //  GenPadding(field, &init_list, &padding_id, PaddingInitializer);
      //}
    }

    code_.SetValue("ARG_LIST", arg_list);
    code_.SetValue("INIT_LIST", init_list);
    code_ += "  pub fn new({{ARG_LIST}}) -> Self {";
    code_ += "    {{STRUCT_NAME}} {";
    code_ += "{{INIT_LIST}}";
    padding_id = 0;
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (field.padding) {
        std::string padding;
        GenPadding(field, &padding, &padding_id, PaddingInitializer);
        code_ += padding;
      }
    }
    code_ += "    }";
    code_ += "  }";

    // Generate accessor methods of the form:
    // type name() const { return flatbuffers::endian_scalar(name_); }
    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;

      //auto field_type = GenTypeGet(field.value.type, " ", "&", "", true);
      auto field_type = GenBuilderArgsAddFuncType(field, "");
      auto is_scalar = IsScalar(field.value.type.base_type) &&
                       !IsFloat(field.value.type.base_type);
      auto member = "self." + Name(field) + "_";
      auto value =
          is_scalar ? "flatbuffers::endian_scalar(" + member + ")" : member;

      code_.SetValue("FIELD_NAME", Name(field));
      code_.SetValue("FIELD_TYPE", field_type);
      code_.SetValue("FIELD_VALUE", GenUnderlyingCast(field, true, value));
      code_.SetValue("REF", IsStruct(field.value.type) ? "&" : "");

      GenComment(field.doc_comment, "  ");
      code_ += "  pub fn {{FIELD_NAME}}(&self) -> {{FIELD_TYPE}} {";
      code_ += "    {{REF}}{{FIELD_VALUE}}";
      code_ += "  }";

      if (parser_.opts.mutable_buffer) {
        auto mut_field_type = GenTypeGet(field.value.type, " ", "", " ", true);
        code_.SetValue("FIELD_TYPE", mut_field_type);
        if (is_scalar) {
          code_.SetValue("ARG", GenTypeBasic(field.value.type, true));
          code_.SetValue("FIELD_VALUE",
                         GenUnderlyingCast(field, false, "_" + Name(field)));

          code_ += "  fn mutate_{{FIELD_NAME}}(&mut self, _{{FIELD_NAME}}: {{ARG}}) {";
          code_ +=
              "    flatbuffers::write_scalar(&self.{{FIELD_NAME}}_, "
              "{{FIELD_VALUE}});";
          code_ += "  }";
        } else {
          code_ += "  fn mutable_{{FIELD_NAME}}(&mut self) -> &mut {{FIELD_TYPE}}{";
          code_ += "    &mut self.{{FIELD_NAME}}_";
          code_ += "  }";
        }
      }

      // Generate a comparison function for this field if it is a key.
      if (field.key) {
        code_ += "  fn KeyCompareLessThan(&self, o: &{{STRUCT_NAME}}) -> bool {";
        code_ += "    unimplemented!();";
        code_ += "    //self.{{FIELD_NAME}}() < o.{{FIELD_NAME}}()";
        code_ += "  }";
        auto type = GenTypeBasic(field.value.type, false);
        if (parser_.opts.scoped_enums && field.value.type.enum_def &&
            IsScalar(field.value.type.base_type)) {
          type = GenTypeGet(field.value.type, " ", "const ", " *", true);
        }

        code_.SetValue("KEY_TYPE", type);
        code_ += "  fn KeyCompareWithValue(&self, val: {{KEY_TYPE}}) -> isize {";
        code_ += "    let key = self.{{FIELD_NAME}}();";
        code_ += "    (key > val) as isize - (key < val) as isize";
        code_ += "  }";
      }
    }
    code_.SetValue("NATIVE_NAME", Name(struct_def));
    GenOperatorNewDelete(struct_def);
    code_ += "}";

    code_.SetValue("STRUCT_BYTE_SIZE", NumToString(struct_def.bytesize));
    code_ += "// STRUCT_END({{STRUCT_NAME}}, {{STRUCT_BYTE_SIZE}});";
    code_ += "";
  }

  // Set up the correct namespace. Only open a namespace if the existing one is
  // different (closing/opening only what is necessary).
  //
  // The file must start and end with an empty (or null) namespace so that
  // namespaces are properly opened and closed.
  void SetNameSpace(const Namespace *ns) {
    if (cur_name_space_ == ns) { return; }

    // Compute the size of the longest common namespace prefix.
    // If cur_name_space is A::B::C::D and ns is A::B::E::F::G,
    // the common prefix is A::B:: and we have old_size = 4, new_size = 5
    // and common_prefix_size = 2
    size_t old_size = cur_name_space_ ? cur_name_space_->components.size() : 0;
    size_t new_size = ns ? ns->components.size() : 0;

    size_t common_prefix_size = 0;
    while (common_prefix_size < old_size && common_prefix_size < new_size &&
           ns->components[common_prefix_size] ==
               cur_name_space_->components[common_prefix_size]) {
      common_prefix_size++;
    }

    // Close cur_name_space in reverse order to reach the common prefix.
    // In the previous example, D then C are closed.
    for (size_t j = old_size; j > common_prefix_size; --j) {
      code_ += "}  // pub mod " + cur_name_space_->components[j - 1];
    }
    if (old_size != common_prefix_size) { code_ += ""; }

    // open namespace parts to reach the ns namespace
    // in the previous example, E, then F, then G are opened
    for (auto j = common_prefix_size; j != new_size; ++j) {
      code_ += "pub mod " + ns->components[j] + " {";
      code_ += "  #[allow(unused_imports)]";
      code_ += "  use std::mem;";
      code_ += "  #[allow(unused_imports)]";
      code_ += "  use std::marker::PhantomData;";
      code_ += "  #[allow(unused_imports)]";
      code_ += "  #[allow(unreachable_code)]";
      code_ += "  extern crate flatbuffers;";
      code_ += "  #[allow(unused_imports)]";
      code_ += "  use self::flatbuffers::flexbuffers;";
      code_ += "  #[allow(unused_imports)]";
      code_ += "  use std::cmp::Ordering;";
    }
    if (new_size != common_prefix_size) { code_ += ""; }

    cur_name_space_ = ns;
  }
};

}  // namespace cpp

bool GenerateRust(const Parser &parser, const std::string &path,
                  const std::string &file_name) {
  rust::RustGenerator generator(parser, path, file_name);
  return generator.generate();
}

//std::string RustMakeRule(const Parser &parser, const std::string &path,
//                        const std::string &file_name) {
//  const auto filebase =
//      flatbuffers::StripPath(flatbuffers::StripExtension(file_name));
//  const auto included_files = parser.GetIncludedFilesRecursive(file_name);
//  std::string make_rule = GeneratedFileName(path, filebase) + ": ";
//  for (auto it = included_files.begin(); it != included_files.end(); ++it) {
//    make_rule += " " + *it;
//  }
//  return make_rule;
//}

}  // namespace flatbuffers
