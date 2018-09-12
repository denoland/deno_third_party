"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var utils_1 = require("../../utils");
function hasBody(node) {
    if (utils_1.TypeGuards.isBodyableNode(node) && node.hasBody())
        return true;
    if (utils_1.TypeGuards.isBodiedNode(node))
        return true;
    return utils_1.TypeGuards.isInterfaceDeclaration(node) || utils_1.TypeGuards.isClassDeclaration(node) || utils_1.TypeGuards.isEnumDeclaration(node);
}
exports.hasBody = hasBody;
