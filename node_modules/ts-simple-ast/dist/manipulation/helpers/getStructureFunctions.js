"use strict";
/* tslint:disable */
// DO NOT MANUALLY EDIT!! File generated via: npm run code-generate
Object.defineProperty(exports, "__esModule", { value: true });
var objectAssign = require("object-assign");
var getMixinStructureFuncs = require("./getMixinStructureFunctions");
function fromConstructorDeclarationOverload(node) {
    var structure = {};
    objectAssign(structure, getMixinStructureFuncs.fromScopedNode(node));
    return structure;
}
exports.fromConstructorDeclarationOverload = fromConstructorDeclarationOverload;
function fromMethodDeclarationOverload(node) {
    var structure = {};
    objectAssign(structure, getMixinStructureFuncs.fromStaticableNode(node));
    objectAssign(structure, getMixinStructureFuncs.fromAbstractableNode(node));
    objectAssign(structure, getMixinStructureFuncs.fromScopedNode(node));
    return structure;
}
exports.fromMethodDeclarationOverload = fromMethodDeclarationOverload;
function fromFunctionDeclarationOverload(node) {
    var structure = {};
    objectAssign(structure, getMixinStructureFuncs.fromAmbientableNode(node));
    objectAssign(structure, getMixinStructureFuncs.fromExportableNode(node));
    return structure;
}
exports.fromFunctionDeclarationOverload = fromFunctionDeclarationOverload;
