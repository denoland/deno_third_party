"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var errors = require("../../errors");
/**
 * Verifies to see if an index or negative index exists within a specified length.
 * @param index - Index.
 * @param length - Length index could be in.
 */
function verifyAndGetIndex(index, length) {
    var newIndex = index < 0 ? length + index : index;
    if (newIndex < 0)
        throw new errors.InvalidOperationError("Invalid index: The max negative index is " + length * -1 + ", but " + index + " was specified.");
    if (index > length)
        throw new errors.InvalidOperationError("Invalid index: The max index is " + length + ", but " + index + " was specified.");
    return newIndex;
}
exports.verifyAndGetIndex = verifyAndGetIndex;
