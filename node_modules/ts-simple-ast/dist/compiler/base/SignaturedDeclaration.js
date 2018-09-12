"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var ParameteredNode_1 = require("./ParameteredNode");
var ReturnTypedNode_1 = require("./ReturnTypedNode");
function SignaturedDeclaration(Base) {
    return ReturnTypedNode_1.ReturnTypedNode(ParameteredNode_1.ParameteredNode(Base));
}
exports.SignaturedDeclaration = SignaturedDeclaration;
