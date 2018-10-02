"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("./base");
var JSDocTag_1 = require("./JSDocTag");
exports.JSDocParameterTagBase = base_1.JSDocPropertyLikeTag(JSDocTag_1.JSDocTag);
/**
 * JS doc parameter tag node.
 */
var JSDocParameterTag = /** @class */ (function (_super) {
    tslib_1.__extends(JSDocParameterTag, _super);
    function JSDocParameterTag() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return JSDocParameterTag;
}(exports.JSDocParameterTagBase));
exports.JSDocParameterTag = JSDocParameterTag;
