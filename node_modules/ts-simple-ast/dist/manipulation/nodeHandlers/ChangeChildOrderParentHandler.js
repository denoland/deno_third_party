"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var NodeHandlerHelper_1 = require("./NodeHandlerHelper");
var StraightReplacementNodeHandler_1 = require("./StraightReplacementNodeHandler");
/**
 * Node handler for dealing with a parent who has a child that will change order.
 */
var ChangeChildOrderParentHandler = /** @class */ (function () {
    function ChangeChildOrderParentHandler(compilerFactory, opts) {
        this.compilerFactory = compilerFactory;
        this.straightReplacementNodeHandler = new StraightReplacementNodeHandler_1.StraightReplacementNodeHandler(compilerFactory);
        this.helper = new NodeHandlerHelper_1.NodeHandlerHelper(compilerFactory);
        this.oldIndex = opts.oldIndex;
        this.newIndex = opts.newIndex;
    }
    ChangeChildOrderParentHandler.prototype.handleNode = function (currentNode, newNode, newSourceFile) {
        var currentNodeChildren = this.getChildrenInNewOrder(currentNode.getCompilerChildren());
        var newNodeChildren = newNode.getChildren(newSourceFile);
        errors.throwIfNotEqual(newNodeChildren.length, currentNodeChildren.length, "New children length should match the old children length.");
        for (var i = 0; i < newNodeChildren.length; i++)
            this.helper.handleForValues(this.straightReplacementNodeHandler, currentNodeChildren[i], newNodeChildren[i], newSourceFile);
        this.compilerFactory.replaceCompilerNode(currentNode, newNode);
    };
    ChangeChildOrderParentHandler.prototype.getChildrenInNewOrder = function (children) {
        var result = tslib_1.__spread(children);
        var movingNode = result.splice(this.oldIndex, 1)[0];
        result.splice(this.newIndex, 0, movingNode);
        return result;
    };
    return ChangeChildOrderParentHandler;
}());
exports.ChangeChildOrderParentHandler = ChangeChildOrderParentHandler;
