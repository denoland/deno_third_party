"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expression_1 = require("../expression");
var base_1 = require("./base");
exports.JsxOpeningElementBase = base_1.JsxAttributedNode(base_1.JsxTagNamedNode(expression_1.Expression));
var JsxOpeningElement = /** @class */ (function (_super) {
    tslib_1.__extends(JsxOpeningElement, _super);
    function JsxOpeningElement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return JsxOpeningElement;
}(exports.JsxOpeningElementBase));
exports.JsxOpeningElement = JsxOpeningElement;
