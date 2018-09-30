"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var PrimaryExpression_1 = require("./PrimaryExpression");
exports.ImportExpressionBase = PrimaryExpression_1.PrimaryExpression;
var ImportExpression = /** @class */ (function (_super) {
    tslib_1.__extends(ImportExpression, _super);
    function ImportExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return ImportExpression;
}(exports.ImportExpressionBase));
exports.ImportExpression = ImportExpression;
