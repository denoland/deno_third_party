"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var ArrayUtils_1 = require("../ArrayUtils");
// todo: merge with getNamedNodeByNameOrFindFunction
function getSymbolByNameOrFindFunction(items, nameOrFindFunc) {
    var findFunc;
    if (typeof nameOrFindFunc === "string")
        findFunc = function (dec) { return dec.getName() === nameOrFindFunc; };
    else
        findFunc = nameOrFindFunc;
    return ArrayUtils_1.ArrayUtils.find(items, findFunc);
}
exports.getSymbolByNameOrFindFunction = getSymbolByNameOrFindFunction;
