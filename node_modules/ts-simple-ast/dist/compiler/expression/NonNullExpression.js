"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expressioned_1 = require("./expressioned");
var LeftHandSideExpression_1 = require("./LeftHandSideExpression");
exports.NonNullExpressionBase = expressioned_1.ExpressionedNode(LeftHandSideExpression_1.LeftHandSideExpression);
var NonNullExpression = /** @class */ (function (_super) {
    tslib_1.__extends(NonNullExpression, _super);
    function NonNullExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return NonNullExpression;
}(exports.NonNullExpressionBase));
exports.NonNullExpression = NonNullExpression;
