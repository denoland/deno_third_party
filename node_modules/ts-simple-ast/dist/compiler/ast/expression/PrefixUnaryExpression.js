"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var UnaryExpression_1 = require("./UnaryExpression");
exports.PrefixUnaryExpressionBase = UnaryExpression_1.UnaryExpression;
var PrefixUnaryExpression = /** @class */ (function (_super) {
    tslib_1.__extends(PrefixUnaryExpression, _super);
    function PrefixUnaryExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the operator token of the prefix unary expression.
     */
    PrefixUnaryExpression.prototype.getOperatorToken = function () {
        return this.compilerNode.operator;
    };
    /**
     * Gets the operand of the prefix unary expression.
     */
    PrefixUnaryExpression.prototype.getOperand = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.operand);
    };
    return PrefixUnaryExpression;
}(exports.PrefixUnaryExpressionBase));
exports.PrefixUnaryExpression = PrefixUnaryExpression;
