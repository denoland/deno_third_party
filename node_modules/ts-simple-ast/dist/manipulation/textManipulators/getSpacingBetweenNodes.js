"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var formatting_1 = require("../formatting");
function getSpacingBetweenNodes(opts) {
    var parent = opts.parent, previousSibling = opts.previousSibling, nextSibling = opts.nextSibling, newLineKind = opts.newLineKind, getSiblingFormatting = opts.getSiblingFormatting;
    if (previousSibling == null || nextSibling == null)
        return "";
    var previousSiblingFormatting = getSiblingFormatting(parent, previousSibling);
    var nextSiblingFormatting = getSiblingFormatting(parent, nextSibling);
    if (previousSiblingFormatting === formatting_1.FormattingKind.Blankline || nextSiblingFormatting === formatting_1.FormattingKind.Blankline)
        return newLineKind + newLineKind;
    else if (previousSiblingFormatting === formatting_1.FormattingKind.Newline || nextSiblingFormatting === formatting_1.FormattingKind.Newline)
        return newLineKind;
    else if (previousSiblingFormatting === formatting_1.FormattingKind.Space || nextSiblingFormatting === formatting_1.FormattingKind.Space)
        return " ";
    else
        return "";
}
exports.getSpacingBetweenNodes = getSpacingBetweenNodes;
