"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var errors = require("../../errors");
var FormattingKind_1 = require("./FormattingKind");
function getFormattingKindText(formattingKind, opts) {
    switch (formattingKind) {
        case FormattingKind_1.FormattingKind.Space:
            return " ";
        case FormattingKind_1.FormattingKind.Newline:
            return opts.newLineKind;
        case FormattingKind_1.FormattingKind.Blankline:
            return opts.newLineKind + opts.newLineKind;
        case FormattingKind_1.FormattingKind.None:
            return "";
        default:
            throw new errors.NotImplementedError("Not implemented formatting kind: " + formattingKind);
    }
}
exports.getFormattingKindText = getFormattingKindText;
