"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expressioned_1 = require("./expressioned");
var UnaryExpression_1 = require("./UnaryExpression");
exports.AwaitExpressionBase = expressioned_1.UnaryExpressionedNode(UnaryExpression_1.UnaryExpression);
var AwaitExpression = /** @class */ (function (_super) {
    tslib_1.__extends(AwaitExpression, _super);
    function AwaitExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return AwaitExpression;
}(exports.AwaitExpressionBase));
exports.AwaitExpression = AwaitExpression;
