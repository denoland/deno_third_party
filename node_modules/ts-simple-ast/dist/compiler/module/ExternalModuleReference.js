"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var utils_1 = require("../../utils");
var common_1 = require("../common");
var ExternalModuleReference = /** @class */ (function (_super) {
    tslib_1.__extends(ExternalModuleReference, _super);
    function ExternalModuleReference() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the expression or undefined of the yield expression.
     */
    ExternalModuleReference.prototype.getExpression = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.expression);
    };
    /**
     * Gets the expression of the yield expression or throws if it does not exist.
     */
    ExternalModuleReference.prototype.getExpressionOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getExpression(), "Expected to find an expression.");
    };
    /**
     * Gets the source file referenced or throws if it can't find it.
     */
    ExternalModuleReference.prototype.getReferencedSourceFileOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getReferencedSourceFile(), "Expected to find the referenced source file.");
    };
    /**
     * Gets if the external module reference is relative.
     */
    ExternalModuleReference.prototype.isRelative = function () {
        var expression = this.getExpression();
        if (expression == null || !utils_1.TypeGuards.isStringLiteral(expression))
            return false;
        return utils_1.ModuleUtils.isModuleSpecifierRelative(expression.getLiteralText());
    };
    /**
     * Gets the source file referenced or returns undefined if it can't find it.
     */
    ExternalModuleReference.prototype.getReferencedSourceFile = function () {
        var expression = this.getExpression();
        if (expression == null)
            return undefined;
        var symbol = expression.getSymbol();
        if (symbol == null)
            return undefined;
        return utils_1.ModuleUtils.getReferencedSourceFileFromSymbol(symbol);
    };
    return ExternalModuleReference;
}(common_1.Node));
exports.ExternalModuleReference = ExternalModuleReference;
