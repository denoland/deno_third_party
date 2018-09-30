"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expressioned_1 = require("./expressioned");
var PropertyAccessExpression_1 = require("./PropertyAccessExpression");
exports.SuperPropertyAccessExpressionBase = expressioned_1.SuperExpressionedNode(PropertyAccessExpression_1.PropertyAccessExpression);
var SuperPropertyAccessExpression = /** @class */ (function (_super) {
    tslib_1.__extends(SuperPropertyAccessExpression, _super);
    function SuperPropertyAccessExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return SuperPropertyAccessExpression;
}(exports.SuperPropertyAccessExpressionBase));
exports.SuperPropertyAccessExpression = SuperPropertyAccessExpression;
