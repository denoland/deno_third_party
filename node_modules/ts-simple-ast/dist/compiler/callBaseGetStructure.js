"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/* barrel:ignore */
var objectAssign = require("object-assign");
// todo: add code verification to ensure all fill functions call this
/** @internal */
function callBaseGetStructure(basePrototype, node, structure) {
    var newStructure;
    if (basePrototype.getStructure != null)
        newStructure = basePrototype.getStructure.call(node);
    else
        newStructure = {};
    objectAssign(newStructure, structure);
    return newStructure;
}
exports.callBaseGetStructure = callBaseGetStructure;
