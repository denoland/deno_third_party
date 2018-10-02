"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var common_1 = require("../common");
var base_1 = require("./base");
exports.JsxClosingElementBase = base_1.JsxTagNamedNode(common_1.Node);
var JsxClosingElement = /** @class */ (function (_super) {
    tslib_1.__extends(JsxClosingElement, _super);
    function JsxClosingElement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return JsxClosingElement;
}(exports.JsxClosingElementBase));
exports.JsxClosingElement = JsxClosingElement;
