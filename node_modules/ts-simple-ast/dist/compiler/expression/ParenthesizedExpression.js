"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
var expressioned_1 = require("./expressioned");
exports.ParenthesizedExpressionBase = expressioned_1.ExpressionedNode(Expression_1.Expression);
var ParenthesizedExpression = /** @class */ (function (_super) {
    tslib_1.__extends(ParenthesizedExpression, _super);
    function ParenthesizedExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return ParenthesizedExpression;
}(exports.ParenthesizedExpressionBase));
exports.ParenthesizedExpression = ParenthesizedExpression;
