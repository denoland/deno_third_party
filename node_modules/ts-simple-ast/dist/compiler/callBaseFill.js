"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
// todo: add code verification to ensure all fill functions call this
/** @internal */
function callBaseFill(basePrototype, node, structure) {
    if (basePrototype.fill != null)
        basePrototype.fill.call(node, structure);
}
exports.callBaseFill = callBaseFill;
