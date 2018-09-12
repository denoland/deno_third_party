"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var typescript_1 = require("../../typescript");
function getSyntaxKindName(kind) {
    return getKindCache()[kind];
}
exports.getSyntaxKindName = getSyntaxKindName;
var kindCache = undefined;
function getKindCache() {
    var e_1, _a;
    if (kindCache != null)
        return kindCache;
    kindCache = {};
    try {
        // some SyntaxKinds are repeated, so only use the first one
        for (var _b = tslib_1.__values(Object.keys(typescript_1.SyntaxKind).filter(function (k) { return isNaN(parseInt(k, 10)); })), _c = _b.next(); !_c.done; _c = _b.next()) {
            var name = _c.value;
            var value = typescript_1.SyntaxKind[name];
            if (kindCache[value] == null)
                kindCache[value] = name;
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return kindCache;
}
