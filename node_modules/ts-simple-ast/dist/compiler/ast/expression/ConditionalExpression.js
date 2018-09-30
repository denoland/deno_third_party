"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
exports.ConditionalExpressionBase = Expression_1.Expression;
var ConditionalExpression = /** @class */ (function (_super) {
    tslib_1.__extends(ConditionalExpression, _super);
    function ConditionalExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the condition of the conditional expression.
     */
    ConditionalExpression.prototype.getCondition = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.condition);
    };
    /**
     * Gets the question token of the conditional expression.
     */
    ConditionalExpression.prototype.getQuestionToken = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.questionToken);
    };
    /**
     * Gets the when true expression of the conditional expression.
     */
    ConditionalExpression.prototype.getWhenTrue = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.whenTrue);
    };
    /**
     * Gets the colon token of the conditional expression.
     */
    ConditionalExpression.prototype.getColonToken = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.colonToken);
    };
    /**
     * Gets the when false expression of the conditional expression.
     */
    ConditionalExpression.prototype.getWhenFalse = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.whenFalse);
    };
    return ConditionalExpression;
}(exports.ConditionalExpressionBase));
exports.ConditionalExpression = ConditionalExpression;
