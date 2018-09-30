"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../../base");
var common_1 = require("../../common");
exports.TemplateTailBase = base_1.LiteralLikeNode(common_1.Node);
var TemplateTail = /** @class */ (function (_super) {
    tslib_1.__extends(TemplateTail, _super);
    function TemplateTail() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return TemplateTail;
}(exports.TemplateTailBase));
exports.TemplateTail = TemplateTail;
