"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var UnaryExpression_1 = require("./UnaryExpression");
exports.PostfixUnaryExpressionBase = UnaryExpression_1.UnaryExpression;
var PostfixUnaryExpression = /** @class */ (function (_super) {
    tslib_1.__extends(PostfixUnaryExpression, _super);
    function PostfixUnaryExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the operator token of the postfix unary expression.
     */
    PostfixUnaryExpression.prototype.getOperatorToken = function () {
        return this.compilerNode.operator;
    };
    /**
     * Gets the operand of the postfix unary expression.
     */
    PostfixUnaryExpression.prototype.getOperand = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.operand);
    };
    return PostfixUnaryExpression;
}(exports.PostfixUnaryExpressionBase));
exports.PostfixUnaryExpression = PostfixUnaryExpression;
