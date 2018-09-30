"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var PrimaryExpression_1 = require("./PrimaryExpression");
exports.LiteralExpressionBase = base_1.LiteralLikeNode(PrimaryExpression_1.PrimaryExpression);
var LiteralExpression = /** @class */ (function (_super) {
    tslib_1.__extends(LiteralExpression, _super);
    function LiteralExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return LiteralExpression;
}(exports.LiteralExpressionBase));
exports.LiteralExpression = LiteralExpression;
