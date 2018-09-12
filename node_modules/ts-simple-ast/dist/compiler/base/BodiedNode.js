"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var callBaseFill_1 = require("../callBaseFill");
var setBodyTextForNode_1 = require("./helpers/setBodyTextForNode");
function BodiedNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getBody = function () {
            var body = this.compilerNode.body;
            if (body == null)
                throw new errors.InvalidOperationError("Bodied node should have a body.");
            return this.getNodeFromCompilerNode(body);
        };
        class_1.prototype.setBodyText = function (textOrWriterFunction) {
            var body = this.getBody();
            setBodyTextForNode_1.setBodyTextForNode(body, textOrWriterFunction);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.bodyText != null)
                this.setBodyText(structure.bodyText);
            return this;
        };
        return class_1;
    }(Base));
}
exports.BodiedNode = BodiedNode;
