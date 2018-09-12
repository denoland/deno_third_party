"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
var setBodyTextForNode_1 = require("./helpers/setBodyTextForNode");
function BodyableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getBodyOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getBody(), "Expected to find the node's body.");
        };
        class_1.prototype.getBody = function () {
            return this.getNodeFromCompilerNodeIfExists(this.compilerNode.body);
        };
        class_1.prototype.setBodyText = function (textOrWriterFunction) {
            this.addBody();
            setBodyTextForNode_1.setBodyTextForNode(this.getBodyOrThrow(), textOrWriterFunction);
            return this;
        };
        class_1.prototype.hasBody = function () {
            return this.compilerNode.body != null;
        };
        class_1.prototype.addBody = function () {
            if (this.hasBody())
                return this;
            var semiColon = this.getLastChildByKind(typescript_1.SyntaxKind.SemicolonToken);
            manipulation_1.insertIntoParentTextRange({
                parent: this,
                insertPos: semiColon == null ? this.getEnd() : semiColon.getStart(),
                newText: this.getWriterWithQueuedIndentation().block().toString(),
                replacing: {
                    textLength: semiColon == null ? 0 : semiColon.getFullWidth()
                }
            });
            return this;
        };
        class_1.prototype.removeBody = function () {
            var body = this.getBody();
            if (body == null)
                return this;
            manipulation_1.insertIntoParentTextRange({
                parent: this,
                insertPos: body.getPos(),
                newText: ";",
                replacing: {
                    textLength: body.getFullWidth()
                }
            });
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
exports.BodyableNode = BodyableNode;
