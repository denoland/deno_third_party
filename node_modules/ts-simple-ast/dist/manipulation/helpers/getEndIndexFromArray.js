"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/**
 * Gets the end index from a possibly undefined array.
 * @param array - Array that could possibly be undefined.
 */
function getEndIndexFromArray(array) {
    return array == null ? 0 : array.length;
}
exports.getEndIndexFromArray = getEndIndexFromArray;
