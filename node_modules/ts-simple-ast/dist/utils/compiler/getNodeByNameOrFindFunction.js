"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var ArrayUtils_1 = require("../ArrayUtils");
function getNodeByNameOrFindFunction(items, nameOrFindFunc) {
    var findFunc;
    if (typeof nameOrFindFunc === "string")
        findFunc = function (dec) { return dec.getName != null && dec.getName() === nameOrFindFunc; };
    else
        findFunc = nameOrFindFunc;
    return ArrayUtils_1.ArrayUtils.find(items, findFunc);
}
exports.getNodeByNameOrFindFunction = getNodeByNameOrFindFunction;
function getNotFoundErrorMessageForNameOrFindFunction(findName, nameOrFindFunction) {
    if (typeof nameOrFindFunction === "string")
        return "Expected to find " + findName + " named '" + nameOrFindFunction + "'.";
    return "Expected to find " + findName + " that matched the provided condition.";
}
exports.getNotFoundErrorMessageForNameOrFindFunction = getNotFoundErrorMessageForNameOrFindFunction;
