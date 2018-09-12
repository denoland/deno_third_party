"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var base_1 = require("../base");
var statement_1 = require("../statement");
function FunctionLikeDeclaration(Base) {
    return base_1.JSDocableNode(base_1.TypeParameteredNode(base_1.SignaturedDeclaration(statement_1.StatementedNode(base_1.ModifierableNode(Base)))));
}
exports.FunctionLikeDeclaration = FunctionLikeDeclaration;
