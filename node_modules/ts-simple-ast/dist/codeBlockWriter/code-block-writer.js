"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var code_block_writer_1 = require("code-block-writer");
// this is a trick to get the import module defined in the local scope by its name, but have the compiler
// understand this as exporting the ambient declaration above (so it works at compile time and run time)
// @ts-ignore: Implicit use of this.
var tempThis = this;
tempThis["CodeBlockWriter"] = code_block_writer_1.default;
