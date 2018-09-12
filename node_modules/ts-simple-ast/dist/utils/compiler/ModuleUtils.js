"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var typescript_1 = require("../../typescript");
var StringUtils_1 = require("../StringUtils");
var ModuleUtils = /** @class */ (function () {
    function ModuleUtils() {
    }
    ModuleUtils.isModuleSpecifierRelative = function (text) {
        return StringUtils_1.StringUtils.startsWith(text, "./")
            || StringUtils_1.StringUtils.startsWith(text, "../");
    };
    ModuleUtils.getReferencedSourceFileFromSymbol = function (symbol) {
        var declarations = symbol.getDeclarations();
        if (declarations.length === 0 || declarations[0].getKind() !== typescript_1.SyntaxKind.SourceFile)
            return undefined;
        return declarations[0];
    };
    return ModuleUtils;
}());
exports.ModuleUtils = ModuleUtils;
