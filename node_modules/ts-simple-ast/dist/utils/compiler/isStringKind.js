"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var typescript_1 = require("../../typescript");
/**
 * Gets if the kind is a string kind.
 * @param kind - Node kind.
 */
function isStringKind(kind) {
    switch (kind) {
        case typescript_1.SyntaxKind.StringLiteral:
        case typescript_1.SyntaxKind.NoSubstitutionTemplateLiteral:
        case typescript_1.SyntaxKind.TemplateHead:
        case typescript_1.SyntaxKind.TemplateMiddle:
        case typescript_1.SyntaxKind.TemplateTail:
            return true;
        default:
            return false;
    }
}
exports.isStringKind = isStringKind;
