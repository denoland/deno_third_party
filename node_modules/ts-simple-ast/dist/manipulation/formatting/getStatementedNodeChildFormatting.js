"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var FormattingKind_1 = require("./FormattingKind");
var hasBody_1 = require("./hasBody");
function getStatementedNodeChildFormatting(parent, member) {
    if (hasBody_1.hasBody(member))
        return FormattingKind_1.FormattingKind.Blankline;
    return FormattingKind_1.FormattingKind.Newline;
}
exports.getStatementedNodeChildFormatting = getStatementedNodeChildFormatting;
function getClausedNodeChildFormatting(parent, member) {
    return FormattingKind_1.FormattingKind.Newline;
}
exports.getClausedNodeChildFormatting = getClausedNodeChildFormatting;
