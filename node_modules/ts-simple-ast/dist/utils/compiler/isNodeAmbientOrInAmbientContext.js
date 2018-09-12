"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var typescript_1 = require("../../typescript");
var TypeGuards_1 = require("../TypeGuards");
function isNodeAmbientOrInAmbientContext(node) {
    var e_1, _a;
    if (checkNodeIsAmbient(node) || node.sourceFile.isDeclarationFile())
        return true;
    try {
        for (var _b = tslib_1.__values(node.getAncestorsIterator(false)), _c = _b.next(); !_c.done; _c = _b.next()) {
            var ancestor = _c.value;
            if (checkNodeIsAmbient(ancestor))
                return true;
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return false;
}
exports.isNodeAmbientOrInAmbientContext = isNodeAmbientOrInAmbientContext;
function checkNodeIsAmbient(node) {
    var isThisAmbient = (node.getCombinedModifierFlags() & typescript_1.ts.ModifierFlags.Ambient) === typescript_1.ts.ModifierFlags.Ambient;
    return isThisAmbient || TypeGuards_1.TypeGuards.isInterfaceDeclaration(node) || TypeGuards_1.TypeGuards.isTypeAliasDeclaration(node);
}
