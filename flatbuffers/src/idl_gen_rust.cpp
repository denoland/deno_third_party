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

// Convert a camelCaseIdentifier or CamelCaseIdentifier to an
// underscore_based_indentifier.
std::string MakeSnakeCase(const std::string &in) {
  std::string s;
  for (size_t i = 0; i < in.length(); i++) {
    if (islower(in[i])) {
      s += static_cast<char>(in[i]);
    } else {
      if (i > 0) {
        s += '_';
      }
      s += static_cast<char>(tolower(in[i]));
    }
  }
  return s;
}

// Convert a string to all uppercase.
std::string MakeUpper(const std::string &in) {
  std::string s;
  for (size_t i = 0; i < in.length(); i++) {
    s += static_cast<char>(toupper(in[i]));
  }
  return s;
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

  std::string EscapeKeywordPath(const std::string &name) const {
    return keywords_.find(name) == keywords_.end() ? name : name + "::";
  }

  std::string Name(const Definition &def) const {
    return EscapeKeyword(def.name);
  }

  std::string Name(const EnumVal &ev) const { return EscapeKeyword(ev.name); }

  std::string WrapInNameSpace(const Definition &def) const {
    return WrapInNameSpace(def.defined_namespace, Name(def));
  }
  std::string WrapInNameSpace(const Namespace *ns,
                              const std::string &name) const {
    if (CurrentNameSpace() == ns) return name;
    std::string prefix = GetRelativeNamespaceTraversal(CurrentNameSpace(), ns);
    return prefix + name;
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

        code_.SetValue("STRUCT_NAME", name);
        code_.SetValue("STRUCT_NAME_SNAKECASE", MakeSnakeCase(name));
        code_.SetValue("STRUCT_NAME_CAPS", MakeUpper(MakeSnakeCase(name)));

        // The root datatype accessors:
        code_ += "#[inline]";
        code_ +=
            "pub fn get_root_as_{{STRUCT_NAME_SNAKECASE}}<'a>(buf: &'a [u8])"
            " -> {{STRUCT_NAME}}<'a> {";
        code_ += "  flatbuffers::get_root::<{{STRUCT_NAME}}<'a>>(buf)";
        code_ += "}";
        code_ += "";

        code_ += "#[inline]";
        code_ +=
            "pub fn get_size_prefixed_root_as_{{STRUCT_NAME_SNAKECASE}}<'a>(buf: &'a [u8])"
            " -> {{STRUCT_NAME}}<'a> {";
        code_ += "  flatbuffers::get_size_prefixed_root::<{{STRUCT_NAME}}<'a>>(buf)";
        code_ += "}";
        code_ += "";

        if (parser_.file_identifier_.length()) {
          // Declare the identifier
          code_ += "pub const {{STRUCT_NAME_CAPS}}_IDENTIFIER: &'static str\\";
          code_ += " = \"" + parser_.file_identifier_ + "\";";
          code_ += "";

          // Check if a buffer has the identifier.
          code_ += "#[inline]";
          code_ += "pub fn {{STRUCT_NAME_SNAKECASE}}_buffer_has_identifier(buf: &[u8])"
                   " -> bool {";
          code_ += "  return flatbuffers::buffer_has_identifier(";
          code_ += "      buf, {{STRUCT_NAME_CAPS}}_IDENTIFIER, false);";
          code_ += "}";
          code_ += "";
          code_ += "#[inline]";
          code_ += "pub fn {{STRUCT_NAME_SNAKECASE}}_size_prefixed_buffer_has_identifier(buf: &[u8])"
                   " -> bool {";
          code_ += "  return flatbuffers::buffer_has_identifier(";
          code_ += "      buf, {{STRUCT_NAME_CAPS}}_IDENTIFIER, true);";
          code_ += "}";
          code_ += "";
        }

        if (parser_.file_extension_.length()) {
          // Return the extension
          code_ += "pub const {{STRUCT_NAME_CAPS}}_EXTENSION: &'static str =\\";
          code_ += " \"" + parser_.file_extension_ + "\";";
          code_ += "";
        }

        // Finish a buffer with a given root object:
        code_.SetValue("OFFSET_TYPELABEL", Name(struct_def) + "Offset");
        code_ += "#[inline]";
        code_ += "pub fn finish_{{STRUCT_NAME_SNAKECASE}}_buffer<'a, 'b>(";
        code_ += "    fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>,";
        code_ += "    root: flatbuffers::Offset<{{STRUCT_NAME}}<'a>>) {";
        if (parser_.file_identifier_.length()) {
          code_ += "  fbb.finish(root, Some({{STRUCT_NAME_CAPS}}_IDENTIFIER));";
        } else {
          code_ += "  fbb.finish(root, None);";
        }
        code_ += "}";
        code_ += "";
        code_ += "#[inline]";
        code_ += "pub fn finish_size_prefixed_{{STRUCT_NAME_SNAKECASE}}_buffer<'a, 'b>(";
        code_ += "    fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>,";
        code_ += "    root: flatbuffers::Offset<{{STRUCT_NAME}}<'a>>) {";
        if (parser_.file_identifier_.length()) {
          code_ += "  fbb.finish_size_prefixed(root, Some({{STRUCT_NAME_CAPS}}_IDENTIFIER));";
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

  std::string GenEnumTypeForDecl(const Type &type) const {
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
    // TODO(rw): convert this to enum switch
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
        return "flatbuffers::Table<" + lifetime + ">";
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
    // TODO(rw): convert this to enum switch
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
    // TODO(rw): convert this to enum switch
    if (IsScalar(type.base_type)) {
      return GenTypeBasic(type, false);
    } else if (IsStruct(type)) {
      return GenTypePointer(type, "");
    } else {
      return "flatbuffers::UOffsetT";
    }
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
        case FullElementType::UnionValue: {
          // vectors of unions are not supported yet
          FLATBUFFERS_ASSERT(false);
          return FullElementType::VectorOfUnionValue;
        }
        default: { FLATBUFFERS_ASSERT(false); }
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
    // TODO(rw): convert this to enum switch
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

  std::string GenEnumValDecl(const EnumDef &enum_def,
                             const std::string &enum_val) const {
    return enum_val;
  }

  std::string GetEnumValUse(const EnumDef &enum_def,
                            const EnumVal &enum_val) const {
    return Name(enum_def) + "::" + Name(enum_val);
    //return Name(enum_val);
    //const IDLOptions &opts = parser_.opts;
    //if (opts.scoped_enums) {
    //  return Name(enum_def) + "::" + Name(enum_val);
    //} else if (opts.prefixed_enums) {
    //  return Name(enum_def) + "_" + Name(enum_val);
    //} else {
    //  return Name(enum_val);
    //}
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

  // Generate an enum declaration,
  // an enum string lookup table,
  // an enum match function,
  // and an enum array of values
  void GenEnum(const EnumDef &enum_def) {
    code_.SetValue("ENUM_NAME", Name(enum_def));
    code_.SetValue("BASE_TYPE", GenEnumTypeForDecl(enum_def.underlying_type));
    code_.SetValue("SEP", "");

    GenComment(enum_def.doc_comment);
    code_ += "#[allow(non_camel_case_types)]";
    code_ += "#[repr({{BASE_TYPE}})]";
    code_ += "#[derive(Clone, Copy, PartialEq, Debug)]";
    code_ += "pub enum " + Name(enum_def) + " {";

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

    code_ += "";
    code_ += "}";
    code_ += "";

    code_.SetValue("ENUM_NAME", Name(enum_def));
    code_.SetValue("ENUM_NAME_SNAKE", MakeSnakeCase(Name(enum_def)));
    code_.SetValue("ENUM_NAME_CAPS", MakeUpper(MakeSnakeCase(Name(enum_def))));
    code_.SetValue("ENUM_MIN_BASE_VALUE", NumToString(minv->value));
    code_.SetValue("ENUM_MAX_BASE_VALUE", NumToString(maxv->value));

    code_ += "const ENUM_MIN_{{ENUM_NAME_CAPS}}:{{BASE_TYPE}} = {{ENUM_MIN_BASE_VALUE}};";
    code_ += "const ENUM_MAX_{{ENUM_NAME_CAPS}}:{{BASE_TYPE}} = {{ENUM_MAX_BASE_VALUE}};";
    code_ += "";
    code_ += "impl<'a> flatbuffers::Follow<'a> for {{ENUM_NAME}} {";
    code_ += "    type Inner = Self;";
    code_ += "    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {";
    code_ += "        flatbuffers::read_scalar_at::<Self>(buf, loc)";
    code_ += "    }";
    code_ += "}";
    code_ += "impl flatbuffers::EndianScalar for {{ENUM_NAME}} {";
    code_ += "    fn to_little_endian(self) -> Self {";
    code_ += "        let n = {{BASE_TYPE}}::to_le(self as {{BASE_TYPE}});";
    code_ += "        let ptr = (&n) as *const {{BASE_TYPE}} as *const {{ENUM_NAME}};";
    code_ += "        unsafe { *ptr }";
    code_ += "    }";
    code_ += "    fn from_little_endian(self) -> Self {";
    code_ += "        let n = {{BASE_TYPE}}::from_le(self as {{BASE_TYPE}});";
    code_ += "        let ptr = (&n) as *const {{BASE_TYPE}} as *const {{ENUM_NAME}};";
    code_ += "        unsafe { *ptr }";
    code_ += "    }";
    code_ += "}";
    code_ += "";

    // Generate an array of all enumeration values
    auto num_fields = NumToString(enum_def.vals.vec.size());
    code_ += "#[allow(non_camel_case_types)]";
    code_ += "const ENUM_VALUES_{{ENUM_NAME_CAPS}}:[{{ENUM_NAME}}; " +
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

    // Generate a string table for enum values.
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
      code_ += "#[allow(non_camel_case_types)]";
      code_ += "const ENUM_NAMES_{{ENUM_NAME_CAPS}}:[&'static str; " +
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

      code_ += "pub fn enum_name_{{ENUM_NAME_SNAKE}}(e: {{ENUM_NAME}}) -> &'static str {";

      code_ += "  let index: usize = e as usize\\";
      if (enum_def.vals.vec.front()->value) {
        auto vals = GetEnumValUse(enum_def, *enum_def.vals.vec.front());
        code_ += " - " + vals + " as usize\\";
      }
      code_ += ";";

      code_ += "  ENUM_NAMES_{{ENUM_NAME_CAPS}}[index]";
      code_ += "}";
      code_ += "";
    }

    if (enum_def.is_union) {
      // Generate tyoesafe offset(s) for unions
      code_.SetValue("NAME", Name(enum_def));
      code_.SetValue("UNION_OFFSET_NAME", Name(enum_def) + "UnionTableOffset");
      code_ += "pub struct {{UNION_OFFSET_NAME}} {}";
    }
  }

  // Generates a value with optionally a cast applied if the field has a
  // different underlying type from its interface type (currently only the
  // case for enums. "from" specify the direction, true meaning from the
  // underlying type to the interface type.
  std::string GenUnderlyingCast(const FieldDef &field, bool from,
                                const std::string &val) {
    //switch (GetFullElementType(field.value.type)) {
    //  case FullElementType::Integer: { return GenDefaultConstant(field); }
    //  case FullElementType::Float: { return GenDefaultConstant(field); }
    //  case FullElementType::Bool: { return field.value.constant == "0" ? "false" : "true"; }
    //  case FullElementType::UnionKey:
    //  case FullElementType::EnumKey: {}
    //  case default: {
    //}
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
      stream << MakeSnakeCase(*d) + "::";
    }
    return stream.str();
  }

  std::string GenDefaultConstant(const FieldDef &field) {
    //assert(false);
    return field.value.type.base_type == BASE_TYPE_FLOAT
               ? field.value.constant + ""
               : field.value.constant;
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
        return "Option<flatbuffers::Offset<" + typname + "<" + lifetime + ">>>";
      }
      case FullElementType::String: {
        return "Option<flatbuffers::Offset<&" + lifetime + " str>>";
      }
      case FullElementType::EnumKey:
      case FullElementType::UnionKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return typname;
      }
      case FullElementType::UnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "Option<flatbuffers::Offset<flatbuffers::UnionMarker>>";
      }

      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ",  " + typname + ">>>";
      }
      case FullElementType::VectorOfBool: {
        const auto typname = GenTypeBasic(type, false);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", bool>>>";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>>";
      }
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>>";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsUOffset<" + typname + "<" + lifetime + ">>>>>";
      }
      case FullElementType::VectorOfString: {
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsUOffset<&" + lifetime + " str>>>>";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def) + "UnionTableOffset";
        return "Option<flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsUOffset<flatbuffers::Table<" + lifetime + ">>>>";
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
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsUOffset<" + typname + "<" + lifetime + ">>>>";
      }
      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
      }
      case FullElementType::VectorOfBool: {
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", bool>>";
      }
      case FullElementType::VectorOfString: {
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsUOffset<&" + lifetime + " str>>>";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", " + typname + ">>";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "flatbuffers::Offset<flatbuffers::Vector<" + lifetime + ", flatbuffers::ForwardsUOffset<flatbuffers::Table<" + lifetime + ">>>";
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
        return "flatbuffers::Offset<" + typname + "<" + lifetime + ">>";
      }
      case FullElementType::Integer:
      case FullElementType::Float: {
        const auto typname = GenTypeBasic(type, false);
        return typname;
      }
      case FullElementType::Bool: {
        return "bool";
      }
      case FullElementType::String: {
        return "flatbuffers::Offset<&" + lifetime + " str>";
      }
      case FullElementType::UnionKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return typname;
      }
      case FullElementType::UnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "flatbuffers::Offset<flatbuffers::UnionMarker>";
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
            return "self.fbb_.push_slot_offset_relative::<" + typname + ">";
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
      case FullElementType::UnionKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return typname;
      }

      case FullElementType::UnionValue: {
        return "Option<flatbuffers::Table<" + lifetime + ">>";
      }
      case FullElementType::String: {
         return "Option<&" + lifetime + " str>";
      }
      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        return "Option<&" + lifetime + " [" + typname + "]>";
      }
      case FullElementType::VectorOfBool: {
        return "Option<&" + lifetime + " [bool]>";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "Option<&" + lifetime + " [" + typname + "]>";
      }
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<&" + lifetime + " [" + typname + "]>";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "Option<flatbuffers::Vector<flatbuffers::ForwardsUOffset<" + typname + "<" + lifetime + ">>>>";
      }
      case FullElementType::VectorOfString: {
        return "Option<flatbuffers::Vector<flatbuffers::ForwardsUOffset<&" + lifetime + " str>>>";
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
    const std::string offset_name = offset_prefix + "::" + GenFieldOffsetName(field);
    const Type& type = field.value.type;

    switch (GetFullElementType(field.value.type)) {
      case FullElementType::Integer:
      case FullElementType::Float:
      case FullElementType::Bool: {
        const auto typname = GenTypeBasic(type, false);
        const std::string default_value = GetDefaultScalarValue(field);
        return "self._tab.get::<" + typname + ">(" + offset_name + ", Some(" + default_value + ")).unwrap()";
      }
      case FullElementType::Struct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "self._tab.get::<&" + lifetime + " " + typname + ">(" + offset_name + ", None)";
      }
      case FullElementType::Table: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "self._tab.get::<flatbuffers::ForwardsUOffset<" + typname + "<" + lifetime + ">>>(" + offset_name + ", None)";
      }
      case FullElementType::UnionValue: {
        return "self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Table<" + lifetime + ">>>(" + offset_name + ", None)";
      }
      case FullElementType::UnionKey:
      case FullElementType::EnumKey: {
        const std::string underlying_typname = GenTypeBasic(type, false);
        const std::string typname = WrapInNameSpace(*type.enum_def);
        const std::string default_value = GetDefaultScalarValue(field);
        return "self._tab.get::<" + typname + ">(" + offset_name + ", Some(" + default_value + ")).unwrap()";
      }
      case FullElementType::String: {
        return "self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(" + offset_name + ", None)";
      }

      case FullElementType::VectorOfInteger:
      case FullElementType::VectorOfFloat: {
        const auto typname = GenTypeBasic(type.VectorType(), false);
        return "self._tab.get::<flatbuffers::ForwardsUOffset<&[" + typname + "]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfBool: {
        return "self._tab.get::<flatbuffers::ForwardsUOffset<&[bool]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfEnumKey: {
        const auto typname = WrapInNameSpace(*type.enum_def);
        return "self._tab.get::<flatbuffers::ForwardsUOffset<&[" + typname + "]>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfStruct: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::SliceOfGeneratedStruct<" + typname + ">>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfTable: {
        const auto typname = WrapInNameSpace(*type.struct_def);
        return "self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<flatbuffers::ForwardsUOffset<" + typname + "<" + lifetime + ">>>>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfString: {
        return "self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<flatbuffers::ForwardsUOffset<&" + lifetime + " str>>>>(" + offset_name + ", None)";
      }
      case FullElementType::VectorOfUnionValue: {
        const auto typname = WrapInNameSpace(*type.enum_def) + "UnionTableOffset";
        return "self._tab.get_slot_vector::<flatbuffers::Offset<" + typname + "<" + lifetime + ">>>(" + offset_name + ")";
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
    code_ += "#[derive(Copy, Clone, Debug, PartialEq)]";
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

    // Generate a convenient CreateX function that uses the above builder
    // to create a table in one go.
    code_.SetValue("MAYBE_UNDERSCORE",
        struct_def.fields.vec.size() == 0 ? "_" : "");
    code_ += "    #[allow(unused_mut)]";
    code_ += "    pub fn create<'x: 'y, 'y: 'z, 'z>(";
    code_ += "        _fbb: &'z mut flatbuffers::FlatBufferBuilder<'x>,";
    code_ += "        {{MAYBE_UNDERSCORE}}args: &'y {{STRUCT_NAME}}Args<'y>) -> \\";
    code_ += "flatbuffers::Offset<{{STRUCT_NAME}}<'x>> {";
    //for (auto it = struct_def.fields.vec.begin();
    //     it != struct_def.fields.vec.end(); ++it) {
    //  const auto &field = **it;
    //  if (!field.deprecated) { GenParam(field, false, ",\n    "); }
    //}

    code_ += "      let mut builder = {{STRUCT_NAME}}Builder::new(_fbb);";
    for (size_t size = struct_def.sortbysize ? sizeof(largest_scalar_t) : 1;
         size; size /= 2) {
      for (auto it = struct_def.fields.vec.rbegin();
           it != struct_def.fields.vec.rend(); ++it) {
        const auto &field = **it;
        // TODO(rw): fully understand this sortbysize usage
        if (!field.deprecated && (!struct_def.sortbysize ||
                                  size == SizeOf(field.value.type.base_type))) {
          code_.SetValue("FIELD_NAME", Name(field));
          if (ElementTypeUsesOption(field.value.type)) {
            code_ += "      if let Some(x) = args.{{FIELD_NAME}} { builder.add_{{FIELD_NAME}}(x); }";
          } else {
            code_ += "      builder.add_{{FIELD_NAME}}(args.{{FIELD_NAME}});";
          }
        }
      }
    }
    code_ += "      builder.finish()";
    code_ += "    }";
    code_ += "";

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
        code_ += "             Some(<flatbuffers::ForwardsUOffset<{{STRUCT_NAME}}<'a>>>::follow(data, 0))";
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
    code_.SetValue("STRUCT_NAME_SNAKECASE", MakeSnakeCase(Name(struct_def)));
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
    code_ += "  pub fn finish(self) -> flatbuffers::Offset<{{STRUCT_NAME}}<'a>> {";
    code_ += "    let o = self.fbb_.end_table(self.start_);";

    for (auto it = struct_def.fields.vec.begin();
         it != struct_def.fields.vec.end(); ++it) {
      const auto &field = **it;
      if (!field.deprecated && field.required) {
        code_.SetValue("FIELD_NAME", MakeSnakeCase(Name(field)));
        code_.SetValue("OFFSET_NAME", GenFieldOffsetName(field));
        code_ += "    self.fbb_.required(o, {{STRUCT_NAME}}::{{OFFSET_NAME}}, \"{{FIELD_NAME}}\");";
      }
    }
    code_ += "    flatbuffers::Offset::new(o.value())";
    code_ += "  }";
    code_ += "}";
    code_ += "";
  }

  std::string GenUnionUnpackVal(const FieldDef &afield,
                                const char *vec_elem_access,
                                const char *vec_type_access) {
    return afield.value.type.enum_def->name + "Union::UnPack(" + "_e" +
           vec_elem_access + ", " + Name(afield) + UnionTypeFieldSuffix() +
           "()" + vec_type_access + ", _resolver)";
  }

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
    code_ += "#[derive(Clone, Copy, /* Default, */ Debug, PartialEq)]";

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
                                true));
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
		code_ += "impl flatbuffers::GeneratedStruct for {{STRUCT_NAME}} {}";
    code_ += "impl<'a> flatbuffers::Follow<'a> for {{STRUCT_NAME}} {";
    code_ += "    type Inner = &'a {{STRUCT_NAME}};";
    code_ += "    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {";
    code_ += "        let this_buf = &buf[loc..loc + ::std::mem::size_of::<{{STRUCT_NAME}}>()];";
    code_ += "        let ptr = this_buf.as_ptr() as *const {{STRUCT_NAME}};";
    code_ += "        unsafe { &*ptr }";
    code_ += "    }";
    code_ += "}";
    code_ += "//impl<'a> flatbuffers::Follow<'a> for &'a [{{STRUCT_NAME}}] {";
    code_ += "//    type Inner = Self;//&'a [{{STRUCT_NAME}}];";
    code_ += "//    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {";
    code_ += "//    //    let this_buf = &buf[loc..loc + ::std::mem::size_of::<{{STRUCT_NAME}}>()];";
    code_ += "//    //    let ptr = this_buf.as_ptr() as *const {{STRUCT_NAME}};";
    code_ += "//    //    unsafe { &*ptr }";
    code_ += "//    //}";
    code_ += "//        let sz = ::std::mem::size_of::<{{STRUCT_NAME}}>();";
    code_ += "//        assert!(sz > 0);";
    code_ += "//        let len = flatbuffers::read_scalar::<flatbuffers::UOffsetT>(&buf[loc..loc + flatbuffers::SIZE_UOFFSET]) as usize;";
    code_ += "//        let data_buf = &buf[loc + flatbuffers::SIZE_UOFFSET..loc + flatbuffers::SIZE_UOFFSET + len * sz];";
    code_ += "//        let ptr = data_buf.as_ptr() as *const {{STRUCT_NAME}};";
    code_ += "//        let s: &'a [{{STRUCT_NAME}}] = unsafe { ::std::slice::from_raw_parts(ptr, len) };";
    code_ += "//        s";
    code_ += "//    }";
    code_ += "//}";

    // Generate GetFullyQualifiedName
    code_ += "";
		code_ += "impl {{STRUCT_NAME}} {";
    GenFullyQualifiedNameGetter(struct_def, Name(struct_def));

    // Generate a default constructor.
    code_ += "  pub fn reset(&mut self) {";
    code_ += "    let ptr = self as *mut {{STRUCT_NAME}};";
    code_ += "    let sz =  ::std::mem::size_of::<({{STRUCT_NAME}})>();";
    code_ += "    unsafe {";
    code_ += "        ::std::ptr::write_bytes(ptr, 0, sz);";
    code_ += "    }";
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
        //init_list += ": flatbuffers::endian_scalar(" + type + "),\n";
        init_list += ": " + arg_name + ".to_little_endian(),\n";
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
          is_scalar ? member + ".from_little_endian()" : member;

      code_.SetValue("FIELD_NAME", Name(field));
      code_.SetValue("FIELD_TYPE", field_type);
      code_.SetValue("FIELD_VALUE", GenUnderlyingCast(field, true, value));
      code_.SetValue("REF", IsStruct(field.value.type) ? "&" : "");

      GenComment(field.doc_comment, "  ");
      code_ += "  pub fn {{FIELD_NAME}}(&self) -> {{FIELD_TYPE}} {";
      code_ += "    {{REF}}{{FIELD_VALUE}}";
      code_ += "  }";

      // Generate a comparison function for this field if it is a key.
      if (field.key) {
        code_ += "  fn key_compare_less_than(&self, o: &{{STRUCT_NAME}}) -> bool {";
        code_ += "    self.{{FIELD_NAME}}() < o.{{FIELD_NAME}}()";
        code_ += "  }";
        auto type = GenTypeBasic(field.value.type, false);
        if (parser_.opts.scoped_enums && field.value.type.enum_def &&
            IsScalar(field.value.type.base_type)) {
          type = GenTypeGet(field.value.type, " ", "const ", " *", true);
        }

        code_.SetValue("KEY_TYPE", type);
        code_ += "  fn key_compare_with_value(&self, val: {{KEY_TYPE}}) -> isize {";
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
      code_ += "pub mod " + MakeSnakeCase(ns->components[j]) + " {";
      code_ += "  #![allow(dead_code)]";
      code_ += "  #![allow(unused_imports)]";
      code_ += "";
      code_ += "  use std::mem;";
      code_ += "  use std::marker::PhantomData;";
      code_ += "  use std::cmp::Ordering;";
      code_ += "";
      code_ += "  extern crate flatbuffers;";
      code_ += "  use self::flatbuffers::EndianScalar;";
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

std::string RustMakeRule(const Parser &parser, const std::string &path,
                         const std::string &file_name) {
  std::string filebase =
      flatbuffers::StripPath(flatbuffers::StripExtension(file_name));
  std::string make_rule = GeneratedFileName(path, filebase) + ": ";

  auto included_files = parser.GetIncludedFilesRecursive(file_name);
  for (auto it = included_files.begin(); it != included_files.end(); ++it) {
    make_rule += " " + *it;
  }
  return make_rule;
}

}  // namespace flatbuffers
