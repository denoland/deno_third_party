"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var PrimaryExpression_1 = require("./PrimaryExpression");
exports.SuperExpressionBase = PrimaryExpression_1.PrimaryExpression;
var SuperExpression = /** @class */ (function (_super) {
    tslib_1.__extends(SuperExpression, _super);
    function SuperExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return SuperExpression;
}(exports.SuperExpressionBase));
exports.SuperExpression = SuperExpression;
