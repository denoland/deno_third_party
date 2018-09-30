"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
exports.BinaryExpressionBase = Expression_1.Expression;
var BinaryExpression = /** @class */ (function (_super) {
    tslib_1.__extends(BinaryExpression, _super);
    function BinaryExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the left side of the binary expression.
     */
    BinaryExpression.prototype.getLeft = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.left);
    };
    /**
     * Gets the operator token of the binary expression.
     */
    BinaryExpression.prototype.getOperatorToken = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.operatorToken);
    };
    /**
     * Gets the right side of the binary expression.
     */
    BinaryExpression.prototype.getRight = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.right);
    };
    return BinaryExpression;
}(exports.BinaryExpressionBase));
exports.BinaryExpression = BinaryExpression;
