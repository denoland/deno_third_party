"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var BinaryExpression_1 = require("./BinaryExpression");
exports.AssignmentExpressionBase = BinaryExpression_1.BinaryExpression;
var AssignmentExpression = /** @class */ (function (_super) {
    tslib_1.__extends(AssignmentExpression, _super);
    function AssignmentExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the operator token of the assignment expression.
     */
    AssignmentExpression.prototype.getOperatorToken = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.operatorToken);
    };
    return AssignmentExpression;
}(exports.AssignmentExpressionBase));
exports.AssignmentExpression = AssignmentExpression;
