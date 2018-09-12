"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var utils_1 = require("../../utils");
var getClassMemberFormatting_1 = require("./getClassMemberFormatting");
var getInterfaceMemberFormatting_1 = require("./getInterfaceMemberFormatting");
var getStatementedNodeChildFormatting_1 = require("./getStatementedNodeChildFormatting");
function getGeneralFormatting(parent, child) {
    // todo: support more
    if (utils_1.TypeGuards.isClassDeclaration(parent))
        return getClassMemberFormatting_1.getClassMemberFormatting(parent, child);
    if (utils_1.TypeGuards.isInterfaceDeclaration(parent))
        return getInterfaceMemberFormatting_1.getInterfaceMemberFormatting(parent, child);
    // todo: don't assume it's a statemented node here
    return getStatementedNodeChildFormatting_1.getStatementedNodeChildFormatting(parent, child);
}
exports.getGeneralFormatting = getGeneralFormatting;
