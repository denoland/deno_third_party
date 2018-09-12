"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var typescript_1 = require("../../typescript");
function getParentSyntaxList(node) {
    var e_1, _a;
    var parent = node.parent;
    if (parent == null)
        return undefined;
    var pos = node.pos, end = node.end;
    try {
        for (var _b = tslib_1.__values(parent.getChildren()), _c = _b.next(); !_c.done; _c = _b.next()) {
            var child = _c.value;
            if (child.pos > end || child === node)
                return undefined;
            if (child.kind === typescript_1.SyntaxKind.SyntaxList && child.pos <= pos && child.end >= end)
                return child;
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return undefined; // shouldn't happen
}
exports.getParentSyntaxList = getParentSyntaxList;
