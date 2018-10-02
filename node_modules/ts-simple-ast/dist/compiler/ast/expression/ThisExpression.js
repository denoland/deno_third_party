"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var PrimaryExpression_1 = require("./PrimaryExpression");
exports.ThisExpressionBase = PrimaryExpression_1.PrimaryExpression;
var ThisExpression = /** @class */ (function (_super) {
    tslib_1.__extends(ThisExpression, _super);
    function ThisExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return ThisExpression;
}(exports.ThisExpressionBase));
exports.ThisExpression = ThisExpression;
