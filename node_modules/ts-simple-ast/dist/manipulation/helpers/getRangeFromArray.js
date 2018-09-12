"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var utils_1 = require("../../utils");
function getRangeFromArray(array, index, length, expectedKind) {
    var e_1, _a;
    var children = array.slice(index, index + length);
    if (children.length !== length)
        throw new errors.NotImplementedError("Unexpected! Inserted " + length + " child/children, but " + children.length + " were inserted.");
    try {
        for (var children_1 = tslib_1.__values(children), children_1_1 = children_1.next(); !children_1_1.done; children_1_1 = children_1.next()) {
            var child = children_1_1.value;
            if (child.getKind() !== expectedKind)
                throw new errors.NotImplementedError("Unexpected! Inserting syntax kind of " + utils_1.getSyntaxKindName(expectedKind) +
                    (", but " + child.getKindName() + " was inserted."));
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (children_1_1 && !children_1_1.done && (_a = children_1.return)) _a.call(children_1);
        }
        finally { if (e_1) throw e_1.error; }
    }
    return children;
}
exports.getRangeFromArray = getRangeFromArray;
