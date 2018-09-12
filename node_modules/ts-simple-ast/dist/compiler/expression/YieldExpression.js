"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var base_1 = require("../base");
var Expression_1 = require("./Expression");
exports.YieldExpressionBase = base_1.GeneratorableNode(Expression_1.Expression);
var YieldExpression = /** @class */ (function (_super) {
    tslib_1.__extends(YieldExpression, _super);
    function YieldExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the expression or undefined of the yield expression.
     */
    YieldExpression.prototype.getExpression = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.expression);
    };
    /**
     * Gets the expression of the yield expression or throws if it does not exist.
     */
    YieldExpression.prototype.getExpressionOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getExpression(), "Expected to find an expression.");
    };
    return YieldExpression;
}(exports.YieldExpressionBase));
exports.YieldExpression = YieldExpression;
