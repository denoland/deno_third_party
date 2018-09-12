"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var utils_1 = require("../../utils");
var FormattingKind_1 = require("./FormattingKind");
function getClassMemberFormatting(parent, member) {
    if (parent.isAmbient())
        return FormattingKind_1.FormattingKind.Newline;
    if (hasBody(member))
        return FormattingKind_1.FormattingKind.Blankline;
    return FormattingKind_1.FormattingKind.Newline;
}
exports.getClassMemberFormatting = getClassMemberFormatting;
function hasBody(node) {
    if (utils_1.TypeGuards.isBodyableNode(node) && node.getBody() != null)
        return true;
    if (utils_1.TypeGuards.isBodiedNode(node))
        return true;
    return false;
}
